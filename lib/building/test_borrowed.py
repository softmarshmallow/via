"""Tests for what someone else's kit asked of lay.py and check.py (docs/plan-kits.md, K10;
examples/borrowed): a door piece that is a whole block (rule 6 as it stands), a flat roof's own
tile (rule 10, `role:roof_flat`), and a building as deep as its two corners (check.py, eaves).

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The kit is Kenney's Modular Buildings' as packs/kenney.toml states it: blocks 1 by 1 in plan and
0.625 high, a corner block, a door block, a flat roof tile 0.106 thick.
"""

import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
L = runpy.run_path(os.path.join(HERE, "lay.py"))
C = runpy.run_path(os.path.join(HERE, "check.py"))
P = C["params_of"]({})
PE = L["ParamError"]

SHARED = {"module": 1.0, "wall": 1.0, "storey": 0.625, "door": 0.25}
HOUSE = [(10000, 10000), (16000, 10000), (16000, 14000), (10000, 14000)]       # six blocks by four
ATTRS = {"storeys": 1, "storey_h": 0.625, "plinth": 0, "porch": 2, "walk_out": 3, "kit": "kenney",
         "doors": [{"storey": 0, "side": 0, "at": 2.5}]}


def row(id_, roles, size=(1000, 1000), high=625, tags=(), shared=None, **params):
    w, t = size
    roles = [roles] if isinstance(roles, str) else roles
    return {"id": id_, "tags": [f"role:{r}" for r in roles] + list(tags), "taxon": "kenney",
            "bounds": [-w // 2, -t // 2, 0, w - w // 2, t - t // 2, high], "params": dict(params),
            "shared": SHARED if shared is None else shared}


def blocks(extra=(), drop=()):
    rows = [row("block", "wall", pitch=1.0),
            row("corner", "corner"),
            row("door", "door", pitch=1.0),
            row("roof_tile", "roof_flat", high=106, pitch=1.0)] + list(extra)
    return [r for r in rows if r["id"] not in drop]


def lay(ring=HOUSE, rows=None, **attrs):
    rows = blocks() if rows is None else rows
    foot = [(p[0], p[1], 10.0) for p in ring]
    return L["plan"]("h", ring, {**ATTRS, **attrs}, 10.0, None, None,
                     kits=lambda w, h: L["kit_pieces"](rows, w, h), foot=foot)


def notes(out, code):
    return [n for n in out["notes"] if n["code"] == code]


class DoorBlock(unittest.TestCase):
    """lay.py rule 6 with a door piece that is a whole block, the doorway through it: what the kit
    states of the doorway (`shared.door`) decides whether the wall closes. No rule was added."""

    def test_the_kits_doorway_and_the_door_block_close_the_wall(self):
        """The doorway's 0.25 m and the margin make a gap of 0.35 m; each stretch beside it is
        shortened to whole blocks, and the door block fills the 1 m left."""
        out = lay()
        self.assertEqual(out["open"], [])
        self.assertEqual(notes(out, "gap_wide"), [])
        (door,) = [i for i in out["instances"] if i["asset"] == "door"]
        self.assertEqual(door["at"], (12500, 10500))
        south = sorted((r["span"], r["count"]) for r in out["runs"] if r["key"].startswith("h_s0_w0_"))
        self.assertEqual([c for _, c in south], [1, 2])

    def test_a_door_block_on_every_blocks_middle_closes(self):
        for at in (1.5, 2.5, 3.5, 4.5):
            self.assertEqual(lay(doors=[{"storey": 0, "side": 0, "at": at}])["open"], [], at)

    def test_unstated_the_doorway_is_a_metre_and_no_place_closes(self):
        """The before (examples/borrowed `handed_over`): with no `shared.door` a doorway is the
        library's 1 m and the gap 1.1 m. Each stretch beside it is shortened to whole blocks, the
        first to none, and the wall leaves 3 m for a door block of 1 m: `gap_wide`, with no
        place to offer, since no gap of 1.1 m is whole blocks."""
        sh = {k: v for k, v in SHARED.items() if k != "door"}
        out = lay(rows=[dict(r, shared=sh) for r in blocks()])
        self.assertEqual(out["open"], [])
        (n,) = notes(out, "gap_wide")
        self.assertEqual((round(n["left"]), round(n["need"]), n["places"]), (3000, 1100, (None, None)))

    def test_a_stretch_shorter_than_a_block_beside_a_door_goes(self):
        """A door block in the bay beside a corner: the 0.325 m between the corner and the gap is
        no piece's length; it goes, and the door block stands against the corner block."""
        out = lay(doors=[{"storey": 0, "side": 0, "at": 1.5}])
        self.assertEqual(out["open"], [])
        self.assertEqual(notes(out, "gap_wide"), [])
        self.assertEqual([(r["key"], r["count"]) for r in out["runs"] if r["key"].startswith("h_s0_w0_")],
                         [("h_s0_w0_1", 3)])

    def test_between_two_blocks_the_gap_is_two_blocks_and_said(self):
        """A door 3 m along: each stretch is shortened to whole blocks, the gap is 2 m, and
        `gap_wide` says so, offering places that close."""
        out = lay(doors=[{"storey": 0, "side": 0, "at": 3.0}])
        self.assertEqual(out["open"], [])
        (n,) = notes(out, "gap_wide")
        self.assertAlmostEqual(n["left"], 2000, delta=1)
        self.assertIsNone(n["beside"])                  # the door block stands in the middle: the widths say it
        self.assertTrue(n["places"])
        for at in n["places"]:
            self.assertEqual(notes(lay(doors=[{"storey": 0, "side": 0, "at": at / 1000}]), "gap_wide"), [], at)

    def test_a_door_block_runs_into_no_corner(self):
        with self.assertRaises(PE) as e:
            lay(doors=[{"storey": 0, "side": 0, "at": 1.0}])
        self.assertIn("runs into a corner", str(e.exception))


PANEL = {"module": 3.0, "wall": 0.3, "storey": 2.7, "door": 1.0}


def panels():
    """The village kit's sizes: a bay 3 m, a half bay, a post 0.3 m, a door piece 1.5 m for a
    doorway of 1 m."""
    return [row("frame", "wall", size=(3000, 300), high=2700, shared=PANEL, pitch=3.0),
            row("half", "filler", size=(1500, 300), high=2700, shared=PANEL, pitch=1.5),
            row("post", "corner", size=(300, 300), high=2700, shared=PANEL),
            row("door", "door", size=(1500, 300), high=2700, shared=PANEL, pitch=1.5)]


class EachSide(unittest.TestCase):
    """lay.py rule 6, "Each side on its own" (K10's review): the open wall beside a door piece is
    judged on each side of the piece as it stands, from the faces of what was laid."""

    HALL = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]

    def hall(self, at):
        return lay(ring=self.HALL, rows=panels(), storey_h=2.7, kit="village",
                   doors=[{"storey": 0, "side": 0, "at": at}])

    def test_a_door_piece_off_the_openings_middle_is_said(self):
        """The review's case: a door 1.24 m along; the 0.39 m between the corner and the gap goes,
        the opening runs from the corner's face, 0.3 m, to the bay at 1.8 m, and the door piece
        stands on 1.24 m, 0.49 to 1.99 m: 0.19 m open before it, said."""
        out = self.hall(1.24)
        self.assertEqual(out["open"], [])
        (n,) = notes(out, "gap_wide")
        self.assertEqual(n["beside"], (190.0, 0))
        self.assertAlmostEqual(n["left"], 1500, delta=1)

    def test_a_door_piece_that_fills_its_opening_is_not_said(self):
        """A door 1.05 m along: the door piece stands from the corner's face to the bay."""
        self.assertEqual(notes(self.hall(1.05), "gap_wide"), [])

    def panels_with(self, frame=None, door_box=None):
        rows = panels()
        if frame:
            rows[0] = row("frame", "wall", size=(frame, 300), high=2700, shared=PANEL, pitch=3.0)
        if door_box:
            rows[3] = dict(rows[3], bounds=[door_box[0], -150, 0, door_box[1], 150, 2700])
        return rows

    def gap(self, rows, doors):
        out = lay(ring=self.HALL, rows=rows, storey_h=2.7, kit="village", doors=doors)
        return [(n["left"], n["beside"]) for n in notes(out, "gap_wide")]

    def test_the_door_piece_is_where_its_box_is_not_its_origin(self):
        """A door piece whose origin is not its box's middle: its box runs 1 m to the asset's -x
        and 0.5 m to +x, and +x runs against the side, so on a door 1.24 m along it stands from
        0.74 to 2.24 m: 0.44 m open between it and the corner's face."""
        self.assertEqual(self.gap(self.panels_with(door_box=(-1000, 500)), [{"storey": 0, "side": 0, "at": 1.24}]),
                         [(1500.0, (440.0, 0))])

    def test_the_faces_are_the_pieces_not_their_pitch(self):
        """A bay 3.1 m long on a 3 m pitch reaches 0.05 m past each end of its stretch: the opening
        at a door 5 m along runs from 3.35 to 6.25 m, and the door piece, 4.25 to 5.75 m, leaves
        0.9 m and 0.5 m."""
        self.assertEqual(self.gap(self.panels_with(frame=3100), [{"storey": 0, "side": 0, "at": 5.0}]),
                         [(2900.0, (900.0, 500.0))])

    def test_a_door_on_another_side_is_not_this_sides(self):
        """Two doors, the first on side 1: side 0's opening is judged by its own door's piece."""
        self.assertEqual(self.gap(panels(), [{"storey": 0, "side": 1, "at": 1.1}, {"storey": 0, "side": 0, "at": 1.24}]),
                         [(1500.0, (190.0, 0))])

    def test_no_place_leaves_a_hole_unsaid(self):
        """Every door place 10 mm apart: wherever the door piece and the wall leave more than the
        margin open on one side of the piece, a word is said."""
        for k in range(105, 600):
            at = k / 100
            try:
                out = self.hall(at)
            except PE:
                continue
            leaf = next(i for i in out["instances"] if i["asset"] == "door")
            # What stands on side 0: the two posts, the runs (each piece fills its pitch), the
            # door piece; the longest stretch none of them covers.
            cover = sorted([(0, 300), (12300, 12600), (leaf["at"][0] - 750, leaf["at"][0] + 750)] +
                           [(r["span"][0] * 44400, r["span"][1] * 44400) for r in out["runs"]
                            if r["key"].startswith("h_s0_w0_")])
            hole, reach = 0, 0
            for lo, hi in cover:
                hole, reach = max(hole, lo - reach), max(reach, hi)
            said = notes(out, "gap_wide") or [o for o in out["open"] if o["side"] == 0]
            if hole > 105:
                self.assertTrue(said, at)


class RoofTile(unittest.TestCase):
    """lay.py rule 10: a flat roof's tile is the kit's `role:roof_flat` piece where it has one."""

    def test_a_flat_roof_is_laid_with_the_roof_tile(self):
        out = lay(roof="flat")
        tiles = [i for i in out["instances"] if i["key"].startswith("h_roof_f")]
        self.assertEqual(len(tiles), 8)                                # four by two inside the walls
        self.assertEqual({i["asset"] for i in tiles}, {"roof_tile"})
        self.assertEqual(notes(out, "roof_none"), [])

    def test_a_floor_tile_is_the_floors_and_the_roof_tile_the_roofs(self):
        rows = blocks(extra=[row("floor_tile", "floor", high=50, pitch=1.0)])
        out = lay(rows=rows, roof="flat")
        floor = {i["asset"] for i in out["instances"] if i["key"].startswith("h_s0_f")}
        roof = {i["asset"] for i in out["instances"] if i["key"].startswith("h_roof_f")}
        self.assertEqual((floor, roof), ({"floor_tile"}, {"roof_tile"}))

    def test_without_a_roof_tile_the_floor_tile_roofs_as_before(self):
        rows = blocks(drop=("roof_tile",), extra=[row("floor_tile", "floor", high=50, pitch=1.0)])
        out = lay(rows=rows, roof="flat")
        self.assertEqual({i["asset"] for i in out["instances"] if i["key"].startswith("h_roof_f")}, {"floor_tile"})

    def test_a_terrace_is_tiled_with_the_roof_tile_of_the_storey_below(self):
        """A storey set back (rule 14): the terrace before it is a roof, tiled with the roof tile
        of the storey it stands on: not its floor tile, and not the storey above's roof tile (K10's
        review: three mutations of which tile a terrace takes survived)."""
        rows = panels() + [row("floor_tile", "floor", size=(1500, 1500), high=50, shared=PANEL, pitch=1.5),
                           row("roof_tile", "roof_flat", size=(1500, 1500), high=100, tags=("storey:ground",),
                               shared=PANEL, pitch=1.5),
                           row("top_tile", "roof_flat", size=(1500, 1500), high=100, tags=("storey:top",),
                               shared=PANEL, pitch=1.5)]
        out = lay(ring=EachSide.HALL, rows=rows, storeys=2, storey_h=2.7, kit="village", roof="flat",
                  doors=[{"storey": 0, "side": 0, "at": 1.05}], setback=[{"storey": 1, "sides": [0], "by": 3.0}],
                  stair={"side": 2, "at": 6.3, "width": 1.3, "run": 4.5})
        terrace = [i["asset"] for i in out["instances"] if i["key"].startswith("h_t1_")]
        roof = {i["asset"] for i in out["instances"] if i["key"].startswith("h_roof_f")}
        self.assertEqual((len(terrace), set(terrace), roof), (8, {"roof_tile"}, {"top_tile"}))

    def test_with_neither_the_roof_is_refused_naming_both(self):
        out = lay(rows=blocks(drop=("roof_tile",)), roof="flat")
        (n,) = notes(out, "roof_none")
        self.assertIn("`role:roof_flat` or `role:floor`", n["why"])


def judged(rows):
    return [(r[0], r[1], r[2], r[3]) for r in C["judge"]([], [], rows, P, []) if r[1] != "checked"]


class CornersDeep(unittest.TestCase):
    """check.py: under stated eaves a slope roofs a building as deep as its two corners."""

    def kit(self, span):
        sh = dict(SHARED, eaves=0.139)
        rows = [row("block", "wall", shared=sh, pitch=1.0), row("corner", "corner", shared=sh)]
        s = int(span * 1000)
        return rows + [row("slope", "roof_slope", size=(1000, s), high=782, shared=sh, pitch=1.0, span=span),
                       row("hip", "roof_hip", size=(s, s), high=782, shared=sh, span=span)]

    def test_two_corners_and_no_block_between_is_a_building(self):
        """Kenney's slope: span 1.139 under eaves of 0.139 roofs a building two blocks deep."""
        self.assertFalse([s for s in judged(self.kit(1.139)) if s[1] == "kit_stated"])

    def test_a_panel_kit_as_deep_as_its_two_corners_is_said(self):
        """K10's review: a kit of panels (0.3 m walls and corners, a 3 m bay) whose slope, under
        eaves of 0.5 m, roofs a building 0.6 m deep: two corners touching. Only a kit of blocks,
        whose corner reaches as far as its bay, has a building of no pieces between them."""
        sh = {"module": 3.0, "wall": 0.3, "storey": 3.0, "door": 1.4, "eaves": 0.5}
        rows = [row("wall", "wall", size=(3000, 300), high=3000, shared=sh, pitch=3.0),
                row("half", "filler", size=(1500, 300), high=3000, shared=sh, pitch=1.5),
                row("corner", "corner", size=(300, 300), high=3000, shared=sh),
                row("slope", "roof_slope", size=(1500, 800), high=1000, shared=sh, pitch=1.5, span=0.8)]
        said = [s for s in judged(rows) if s[1] == "kit_stated"]
        self.assertEqual(len(said), 1)
        self.assertIn("roofs a building 0.6 m deep", said[0][3])

    def test_the_least_depth_is_held_to_its_edge(self):
        """A panel kit's side holds at least one piece: a depth one piece less the slack is not
        said, the slack being what `check` lets every kit measure differ by."""
        p = C["params_of"]({"pitch_slack": 0.25})
        sh = {"module": 1.0, "wall": 0.25, "storey": 3.0, "eaves": 0.5}
        rows = [row("wall", "wall", size=(1000, 250), high=3000, shared=sh, pitch=1.0),
                row("corner", "corner", size=(250, 250), high=3000, shared=sh, reach=0.25),
                row("slope", "roof_slope", size=(1000, 1125), high=1000, shared=sh, pitch=1.0, span=1.125)]
        said = [r for r in C["judge"]([], [], rows, p, []) if r[1] == "kit_stated" and "roofs a building" in r[3]]
        self.assertEqual(said, [])           # 0.75 m between the corners: one piece less the slack

    def test_a_depth_no_building_has_is_still_said(self):
        said = [s for s in judged(self.kit(1.389)) if s[1] == "kit_stated"]
        self.assertEqual(len(said), 1)
        self.assertIn("roofs a building 2.5 m deep", said[0][3])


if __name__ == "__main__":
    unittest.main()
