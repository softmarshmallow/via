"""Tests for lay.py rules 15 and 16 and the mirrored `fill` of rule 8, and for what check.py holds a
kit of posts to: posts and bays, the colonnade, the platform, a kit's unit.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The kit is examples/temple's (packs/order.toml), as `pack.query` gives its rows: one unit of
0.9 m, a column every 2.7 m, walls 0.9 m thick with a bay of 1.8 m.
"""

import json
import math
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
L = runpy.run_path(os.path.join(HERE, "lay.py"))
C = runpy.run_path(os.path.join(HERE, "check.py"))
P = C["params_of"]({})
PE = L["ParamError"]

SHARED = {"unit": 0.9, "wall": 0.9, "storey": 9.0, "door": 1.8, "base": 0.9}


def row(id_, roles, size, high=9000, tags=(), z0=0, shared=None, **params):
    """A pack row: a box `size` (length, thickness) mm in plan, from `z0` up `high`."""
    w, t = size
    roles = [roles] if isinstance(roles, str) else roles
    return {"id": id_, "tags": [f"role:{r}" for r in roles] + list(tags), "taxon": "k",
            "bounds": [-w // 2, -t // 2, z0, w - w // 2, t - t // 2, z0 + high], "params": dict(params),
            "shared": SHARED if shared is None else shared}


def order(column=900, lintel=1800, swap=None):
    """The order kit's rows; `swap` replaces rows by id."""
    swap = swap or {}
    rows = [row("wall", "wall", (1800, 900), pitch=1.8),
            row("wall_niche", "wall", (1800, 900), tags=("variant:niche",), pitch=1.8),
            row("wall_half", "filler", (900, 900), pitch=0.9),
            row("corner", "corner", (900, 900)),
            row("door", "door", (3600, 900), pitch=3.6),
            row("tile", "floor", (900, 900), 300, pitch=0.9),
            row("roof_slope", "roof_slope", (1800, 7200), 2700, pitch=1.8, span=7.2),
            row("roof_gable", "roof_gable", (7200, 900), 2700, span=7.2),
            row("base", "wall", (1800, 900), 900, ("storey:base",), pitch=1.8),
            row("base_corner", "corner", (900, 900), 900, ("storey:base",)),
            row("column", ["post", "corner"], (column, column), tags=("storey:colonnade",)),
            row("lintel", "wall", (lintel, 900), 1800, ("storey:colonnade",), z0=7200, pitch=2.7)]
    return [swap.get(r["id"], r) for r in rows]


# The temple: five column bays by seven and one column's width; its front is side 0.
TEMPLE = [(0, 0), (14400, 0), (14400, 19800), (0, 19800)]
CELLA = [(2700, 2700), (11700, 2700), (11700, 17100), (2700, 17100)]
ATTRS = {"storeys": 1, "storey_h": 9.0, "plinth": 0.9, "porch": 2, "walk_out": 3, "kit": "k", "roof": "gable",
         "colonnade": [{"sides": "all", "depth": 2.7}], "doors": [{"storey": 0, "side": 0, "at": 7.2}]}


def lay(rows=None, ring=TEMPLE, **attrs):
    rows = order() if rows is None else rows
    a = {**ATTRS, **attrs}
    foot = [(p[0], p[1], 10.0) for p in ring]
    return L["plan"]("t", ring, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h), foot=foot)


def stations(run, ring):
    """Where the core stands a run's members, mm along `ring` (rules.md 5.1, `ends = "half"`)."""
    total = round(sum(math.dist(p, q) for p, q in zip(ring, ring[1:] + ring[:1])))
    a, b = run["span"][0] * total, run["span"][1] * total
    return [round(a + (2 * k + 1) * (b - a) / (2 * run["count"])) for k in range(run["count"])]


def colonnade_kit(rows=None):
    return L["kit_pieces"](order() if rows is None else rows, "colonnade", None)


class Bays(unittest.TestCase):
    """lay.py rule 15: a side between two corner posts is whole bays and one post."""

    def test_a_side_closes_when_it_is_whole_bays_and_one_post(self):
        n, joints, near = L["post_bays"](14400, colonnade_kit())
        self.assertEqual((n, joints), (5, [450.0, 3150.0, 5850.0, 8550.0, 11250.0, 13950.0]))
        self.assertEqual(near, (None, None))

    def test_a_side_that_does_not_close_is_told_the_two_lengths_that_would(self):
        n, joints, near = L["post_bays"](14900, colonnade_kit())
        self.assertEqual((n, joints), (0, []))
        self.assertEqual(near, (14400, 17100))
        # shorter than one bay and a post: only the longer length closes
        self.assertEqual(L["post_bays"](3000, colonnade_kit())[2], (None, 3600))

    def test_what_bounds_an_end_decides_where_its_post_is(self):
        """A corner post is taken off the stretch; a face has a post's middle half a post beyond
        it (the piece that stands there); a free end has its own post, half a post inside."""
        kit = colonnade_kit()
        self.assertEqual(L["post_bays"](2700, kit, "corner", "face")[1], [450.0, 3150.0])
        self.assertEqual(L["post_bays"](2700, kit, "face", "corner")[1], [-450.0, 2250.0])
        self.assertEqual(L["post_bays"](3600, kit, "free", "free")[1], [450.0, 3150.0])

    def test_a_post_of_another_group_does_not_make_a_kit_of_posts(self):
        """The cella's own kit sees no post (the column is tagged for the colonnade), and a kit
        whose post is untagged beside a bay tagged for one storey is not laid by posts."""
        self.assertFalse(L["by_posts"](L["kit_pieces"](order(), "ground", 9000)))
        self.assertTrue(L["by_posts"](colonnade_kit()))
        mixed = [row("bay", "wall", (2700, 300), tags=("storey:ground",), pitch=3.0), row("post", "post", (300, 300))]
        self.assertFalse(L["by_posts"](L["kit_pieces"](mixed, "ground", None)))
        self.assertFalse(L["by_posts"](None))


class PostWalls(unittest.TestCase):
    """lay.py rule 15: the panels, the posts between them, a door in one bay."""

    # A timber kit of posts: a 3 m bay between posts 0.3 m square, panels 2.7 m, a door frame.
    ROWS = [row("panel", "wall", (2700, 300), 3000, pitch=3.0, shared={}),
            row("panel_window", "wall", (2700, 300), 3000, ("variant:window",), pitch=3.0, shared={}),
            row("post", "post", (300, 300), 3000, shared={}),
            row("frame", "door", (2700, 300), 3000, pitch=3.0, shared={})]
    HALL = [(0, 0), (15300, 0), (15300, 9300), (0, 9300)]        # five bays by three, and a post

    def walls(self, doors=(), picks=None, ring=None, rows=None, crossers=()):
        ring = ring or self.HALL
        kit = L["kit_pieces"](self.ROWS if rows is None else rows)
        return L["wall_plan"](ring, ring, list(doors), kit, "h_s0", "h_s0", crossers, 100, [], picks, "h")

    def test_one_panel_a_bay_and_a_post_at_every_joint(self):
        runs, insts, opens = self.walls()
        self.assertEqual(opens, [])
        panels = {r["key"]: r for r in runs if "_w" in r["key"]}
        posts = {r["key"]: r for r in runs if "_p" in r["key"]}
        self.assertEqual({k: (r["asset"], r["count"]) for k, r in panels.items()},
                         {"h_s0_w0_0": ("panel", 5), "h_s0_w1_0": ("panel", 3), "h_s0_w2_0": ("panel", 5),
                          "h_s0_w3_0": ("panel", 3)})
        self.assertEqual({k: (r["asset"], r["count"]) for k, r in posts.items()},
                         {"h_s0_p0": ("post", 4), "h_s0_p1": ("post", 2), "h_s0_p2": ("post", 4), "h_s0_p3": ("post", 2)})
        # side 0: panels' middles half way between the posts' middles, which stand every 3 m
        # from the corner post's own middle, 0.15 m along
        self.assertEqual(stations(panels["h_s0_w0_0"], self.HALL), [1650, 4650, 7650, 10650, 13650])
        self.assertEqual(stations(posts["h_s0_p0"], self.HALL), [3150, 6150, 9150, 12150])
        self.assertTrue(all(r["offset"] == 0.15 and r["ends"] == "half" and r["ref"] == "h_s0" for r in runs))
        # a corner is a post, in the corner
        self.assertEqual([(i["key"], i["asset"], i["at"]) for i in insts],
                         [("h_s0_c0", "post", (150, 150)), ("h_s0_c1", "post", (15150, 150)),
                          ("h_s0_c2", "post", (15150, 9150)), ("h_s0_c3", "post", (150, 9150))])

    def test_there_are_no_fillers_and_a_side_off_the_bay_is_open(self):
        ring = [(0, 0), (15800, 0), (15800, 9300), (0, 9300)]
        runs, _, opens = self.walls(ring=ring)
        self.assertEqual(sorted(o["key"] for o in opens), ["h_s0_w0_0", "h_s0_w2_0"])
        o = opens[0]
        self.assertEqual((o["length"], o["near"], o["posts"], o["bay"], o["post"]), (15200, (14700.0, 17700.0), True, 3000, 300))
        self.assertEqual(sorted(r["key"] for r in runs), ["h_s0_p1", "h_s0_p3", "h_s0_w1_0", "h_s0_w3_0"])

    def test_a_door_takes_one_panels_place(self):
        door = {"edge": 0, "s": 7650.0, "width": 1000}
        runs, insts, opens = self.walls([door])
        got = {r["key"]: (r["count"], stations(r, self.HALL)) for r in runs if r["key"].startswith("h_s0_w0")}
        self.assertEqual(got, {"h_s0_w0_0": (2, [1650, 4650]), "h_s0_w0_1": (2, [10650, 13650])})
        leaf = next(i for i in insts if i["key"] == "h_s0_door0")
        self.assertEqual((leaf["asset"], leaf["at"], leaf["face"]), ("frame", (7650, 150), 180.0))
        # every post still stands: the door is between two of them
        self.assertEqual(next(r["count"] for r in runs if r["key"] == "h_s0_p0"), 4)

    def test_a_door_off_its_bays_middle_is_refused_with_the_middles(self):
        with self.assertRaises(PE) as e:
            self.walls([{"edge": 0, "s": 7000.0, "width": 1000}])
        self.assertIn("off the middle of its bay", e.exception.message)
        self.assertIn("1.65, 4.65, 7.65, 10.65, 13.65 m along the side", e.exception.hint)

    def test_a_door_wider_than_a_bay_is_clear_has_a_post_in_it(self):
        with self.assertRaises(PE) as e:
            self.walls([{"edge": 0, "s": 7650.0, "width": 3000}])
        self.assertIn("a post stands in its opening", e.exception.message)
        self.assertIn("2.7 m clear between two posts", e.exception.message)
        # with no door piece, a doorway may stand anywhere it is clear of both posts
        rows = [r for r in self.ROWS if r["id"] != "frame"]
        runs, insts, _ = self.walls([{"edge": 0, "s": 7000.0, "width": 1000}], rows=rows)
        self.assertEqual(sorted(r["count"] for r in runs if r["key"].startswith("h_s0_w0")), [2, 2])
        self.assertFalse([i for i in insts if "door" in i["key"]])

    def test_fill_chooses_every_panel_the_last_too(self):
        picks = L["picks_of"]({"fill": [{"sides": [0], "pattern": ["window"]}]}, 1, 4)
        runs, _, _ = self.walls(picks=picks)
        self.assertEqual(next(r for r in runs if r["key"] == "h_s0_w0_0")["asset"], "panel_window")
        picks = L["picks_of"]({"bays": [{"storey": 0, "side": 0, "at": 14.0, "variant": "window"}]}, 1, 4)
        runs, _, _ = self.walls(picks=picks)
        self.assertEqual(next(r for r in runs if r["key"] == "h_s0_w0_0")["assets"],
                         ["panel", "panel", "panel", "panel", "panel_window"])

    def test_a_path_that_is_no_door_takes_the_panels_and_posts_it_crosses(self):
        """A flight along the wall line through the second joint of side 0: the two bays either
        side of that joint have no panel, and the joint has no post."""
        path = [(6150, -2000), (6150, 2000)]
        runs, _, _ = self.walls(crossers=[(path, 1000, ("flight", "f0"))])
        got = {r["key"]: (r["count"], stations(r, self.HALL)) for r in runs if r["key"].startswith("h_s0_w0")
               or r["key"].startswith("h_s0_p0")}
        self.assertEqual(got, {"h_s0_w0_0": (1, [1650]), "h_s0_w0_1": (2, [10650, 13650]),
                               "h_s0_p0": (1, [3150]), "h_s0_p0_1": (2, [9150, 12150])})

    def test_a_door_in_the_first_bay_of_a_building_laid_by_posts(self):
        """The whole plan: a door in the bay beside a corner is between two posts, and is not
        held to the rule for a door cut from a wall of whole pieces (rule 6)."""
        a = {"storeys": 1, "storey_h": 3.0, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k",
             "doors": [{"storey": 0, "side": 0, "at": 1.65, "width": 1}]}
        out = L["plan"]("h", self.HALL, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](self.ROWS, w, h))
        self.assertEqual(out["open"], [])
        leaf = next(i for i in out["instances"] if i["key"] == "h_s0_door0")
        self.assertEqual((leaf["at"], leaf["surface"]), ((1650, 150), "h_s0.floor"))
        self.assertEqual(next(r["count"] for r in out["runs"] if r["key"] == "h_s0_w0_0"), 4)
        # with no door piece a doorway 1 m wide stands anywhere clear of both posts: 0.82 m along,
        # it is 0.02 m off the corner post, where a wall of whole pieces would keep a margin
        rows = [r for r in self.ROWS if r["id"] != "frame"]
        a["doors"] = [{"storey": 0, "side": 0, "at": 0.82, "width": 1}]
        out = L["plan"]("h", self.HALL, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertEqual(next(r["count"] for r in out["runs"] if r["key"] == "h_s0_w0_0"), 4)
        self.assertEqual(out["doorways"][0]["path"][-1], (820, 0))

    def test_an_inside_corner_is_refused(self):
        ell = [(0, 0), (15300, 0), (15300, 6300), (6300, 6300), (6300, 9300), (0, 9300)]
        with self.assertRaises(PE) as e:
            self.walls(ring=ell)
        self.assertIn("no inside corner yet", e.exception.message)

    def test_a_free_end_has_its_own_post(self):
        kit = L["kit_pieces"](self.ROWS)
        runs, insts, opens = L["post_walls"](self.HALL, self.HALL, [], kit, "h", "h_s0",
                                             parts={0: [(3000.0, 9300.0, "free", "free")]})
        self.assertEqual(opens, [])
        self.assertEqual([(r["key"], r["count"], stations(r, self.HALL)) for r in runs],
                         [("h_w0_0", 2, [4650, 7650]), ("h_p0", 1, [6150])])
        self.assertEqual([(i["key"], i["at"]) for i in insts], [("h_p0_a", (3150, 150)), ("h_p0_b", (9150, 150))])


class Colonnade(unittest.TestCase):
    """lay.py rule 16: the walls inside the outline, posts along it, one roof over both."""

    def test_the_walls_stand_inside_and_the_platform_is_the_whole_outline(self):
        out = lay()
        self.assertEqual(out["storeys"][0]["region"], CELLA)
        self.assertEqual(out["platform"], {"key": "t_platform", "region": TEMPLE, "level": 10.9})
        self.assertEqual(out["roof"]["region"], TEMPLE)
        self.assertEqual(out["open"], [])
        self.assertFalse(out["notes"])
        # the cella's walls: the kit's whole pieces on its own outline
        walls = {r["key"]: r["count"] for r in out["runs"] if r["key"].startswith("t_s0_w")}
        self.assertEqual(walls, {"t_s0_w0_0": 1, "t_s0_w0_1": 1, "t_s0_w1_0": 7, "t_s0_w2_0": 4, "t_s0_w3_0": 7})
        self.assertTrue(all(r["ref"] == "t_s0" for r in out["runs"] if r["key"].startswith("t_s0_")))

    def test_a_column_every_bay_and_a_lintel_between_each_two(self):
        out = lay()
        col = [r for r in out["runs"] if r["key"].startswith("t_col_")]
        self.assertTrue(all(r["ref"] == "t_platform" and r["offset"] == 0.45 for r in col))
        self.assertEqual({r["key"]: (r["asset"], r["count"]) for r in col},
                         {"t_col_w0_0": ("lintel", 5), "t_col_p0": ("column", 4), "t_col_w1_0": ("lintel", 7),
                          "t_col_p1": ("column", 6), "t_col_w2_0": ("lintel", 5), "t_col_p2": ("column", 4),
                          "t_col_w3_0": ("lintel", 7), "t_col_p3": ("column", 6)})
        front = next(r for r in col if r["key"] == "t_col_w0_0")
        self.assertEqual(stations(front, TEMPLE), [1800, 4500, 7200, 9900, 12600])
        self.assertEqual(stations(next(r for r in col if r["key"] == "t_col_p0"), TEMPLE), [3150, 5850, 8550, 11250])
        corners = [(i["asset"], i["at"], i["surface"]) for i in out["instances"] if i["key"].startswith("t_col_c")]
        self.assertEqual(corners, [("column", (450, 450), "t_platform.floor"), ("column", (13950, 450), "t_platform.floor"),
                                   ("column", (13950, 19350), "t_platform.floor"), ("column", (450, 19350), "t_platform.floor")])

    def test_a_door_comes_onto_the_platform_and_then_through_the_wall_on_one_axis(self):
        out = lay()
        ways = {d["key"]: (d["path"], d["ends_at"]) for d in out["doorways"]}
        self.assertEqual(ways, {"t_d0": ([(7200, -2000), (7200, 0)], "t_platform.floor"),
                                "t_d0_in": ([(7200, 1700), (7200, 2700)], "t_s0.floor")})
        self.assertEqual([(p["key"], p["ends_at"]) for p in out["footpaths"]], [("t_p0", "t_d0.start")])
        leaf = next(i for i in out["instances"] if i["key"] == "t_s0_door0")
        self.assertEqual((leaf["asset"], leaf["at"], leaf["surface"]), ("door", (7200, 3150), "t_s0.floor"))
        # the lintel over the doorway is unbroken: the front's five lintels are one run
        self.assertEqual(next(r["count"] for r in out["runs"] if r["key"] == "t_col_w0_0"), 5)

    def test_a_door_where_a_column_stands_is_refused_with_the_bays_middles(self):
        with self.assertRaises(PE) as e:
            lay(doors=[{"storey": 0, "side": 0, "at": 5.85}])
        self.assertEqual(e.exception.key, "doors")
        self.assertIn("a post of the colonnade stands 5.85 m along it", e.exception.message)
        # only the middle bay is also a place the wall behind takes its 3.6 m door frame
        self.assertEqual(e.exception.hint, "put the door's middle between two posts: 7.2 m along the side, where the "
                         "wall behind takes it too")
        # as wide as the clear between two columns, on the bay's middle: allowed (no margin is cut)
        lay(doors=[{"storey": 0, "side": 0, "at": 7.2, "width": 1.8}])
        with self.assertRaises(PE):
            lay(doors=[{"storey": 0, "side": 0, "at": 7.2, "width": 1.9}])

    def test_the_walls_door_is_held_to_the_walls_own_corners(self):
        """9.9 m along the front is a bay's middle of the colonnade, and 7.2 m along the wall
        behind it, which is 9 m long: the door frame, 3.6 m, would run into the wall's corner."""
        with self.assertRaises(PE) as e:
            lay(doors=[{"storey": 0, "side": 0, "at": 9.9}])
        self.assertEqual(e.exception.message, "door 0 is 9.9 m along side 0: its opening (3.6 m) runs into a corner "
                         "of the wall behind the colonnade, which takes 0.9 m of that wall")
        self.assertIn("between two posts: 7.2 m along the side", e.exception.hint)

    def test_a_door_beside_the_room_is_refused(self):
        with self.assertRaises(PE) as e:
            lay(doors=[{"storey": 0, "side": 0, "at": 1.8}])
        self.assertIn("the wall behind the colonnade stands from 2.7 to 11.7 m", e.exception.message)

    def test_the_base_goes_round_the_outline_on_the_platform(self):
        out = lay()
        base = [r for r in out["runs"] if r["key"].startswith("t_base_w")]
        self.assertEqual(sorted((r["key"], r["count"], r["ref"]) for r in base),
                         [("t_base_w0_0", 7, "t_platform"), ("t_base_w1_0", 10, "t_platform"), ("t_base_w2_0", 7, "t_platform"),
                          ("t_base_w3_0", 10, "t_platform")])
        self.assertEqual({i["surface"] for i in out["instances"] if i["key"].startswith("t_base_c")}, {"t_platform.floor"})

    def test_the_platform_is_tiled_round_the_room_and_the_room_inside_its_walls(self):
        out = lay()
        platform = [i for i in out["instances"] if i["key"].startswith("t_platform_f")]
        room = [i for i in out["instances"] if i["key"].startswith("t_s0_f")]
        # inside the colonnade's line 12.6 by 18 m is 14 by 20 tiles; the room takes 10 by 16
        self.assertEqual(len(platform), 14 * 20 - 10 * 16)
        self.assertEqual(len(room), 8 * 14)
        self.assertEqual({i["surface"] for i in platform}, {"t_platform.floor"})
        self.assertFalse([i for i in platform if L["inside"](CELLA, i["at"])])
        self.assertTrue(all(L["inside"](CELLA, i["at"]) for i in room))

    def test_the_roof_covers_the_outline(self):
        out = lay()
        eaves = {r["key"]: r["count"] for r in out["runs"] if r["key"].startswith("t_roof_e")}
        self.assertEqual(eaves, {"t_roof_e1": 10, "t_roof_e3": 10})           # 19.8 m less a column at each end
        gables = [i["at"] for i in out["instances"] if i["key"].startswith("t_roof_g")]
        self.assertEqual(sorted(gables), [(3600, 450), (3600, 19350), (10800, 450), (10800, 19350)])

    def test_the_roofs_ends_stand_on_the_colonnade_not_on_the_walls_behind_it(self):
        """A cella with walls half as thick: the eave is still the outline less a column's width
        at each end, and the pediment still stands over the columns."""
        shared = dict(SHARED, wall=0.45)
        thin = {"wall": row("wall", "wall", (1800, 450), pitch=1.8), "wall_half": row("wall_half", "filler", (900, 450), pitch=0.9),
                "wall_niche": row("wall_niche", "wall", (1800, 450), tags=("variant:niche",), pitch=1.8),
                "corner": row("corner", "corner", (450, 450)), "door": row("door", "door", (3600, 450), pitch=3.6)}
        out = lay([dict(r, shared=shared) for r in order(swap=thin)])
        self.assertFalse([n for n in out["notes"] if n["code"] == "roof_none"])
        self.assertEqual({r["key"]: r["count"] for r in out["runs"] if r["key"].startswith("t_roof_e")},
                         {"t_roof_e1": 10, "t_roof_e3": 10})
        self.assertEqual(sorted(i["at"] for i in out["instances"] if i["key"].startswith("t_roof_g"))[0], (3600, 450))

    def test_a_door_on_a_side_the_walls_keep_is_one_doorway_placed_along_the_buildings_side(self):
        """Front only: the flank wall starts 2.7 m along the building's side. A door 9.9 m along
        that side is 7.2 m along the wall; one in the first 2.7 m is beside the platform."""
        front = [{"sides": [0], "depth": 2.7}]
        out = lay(colonnade=front, doors=[{"storey": 0, "side": 1, "at": 9.9}])
        self.assertEqual([(d["key"], d["path"], d["ends_at"]) for d in out["doorways"]],
                         [("t_d0", [(16400, 9900), (14400, 9900)], "t_s0.floor")])
        leaf = next(i for i in out["instances"] if i["key"] == "t_s0_door0")
        self.assertEqual(leaf["at"], (13950, 9900))
        self.assertEqual([(p["key"], p["path"]) for p in out["footpaths"]], [("t_p0", [(19400, 9900)])])
        with self.assertRaises(PE) as e:
            lay(colonnade=front, doors=[{"storey": 0, "side": 1, "at": 1.5}])
        self.assertIn("the wall behind the colonnade stands from 2.7 to 19.8 m", e.exception.message)

    def test_a_colonnade_on_the_front_only(self):
        out = lay(colonnade=[{"sides": [0], "depth": 2.7}])
        self.assertEqual(out["storeys"][0]["region"], [(0, 2700), (14400, 2700), (14400, 19800), (0, 19800)])
        self.assertEqual(out["open"], [])
        col = {r["key"]: (r["asset"], r["count"]) for r in out["runs"] if r["key"].startswith("t_col_")}
        # the front's five bays; one bay on each flank, from the corner column to the wall's corner
        self.assertEqual(col, {"t_col_w0_0": ("lintel", 5), "t_col_p0": ("column", 4), "t_col_w1_0": ("lintel", 1),
                               "t_col_w3_0": ("lintel", 1)})
        flank = next(r for r in out["runs"] if r["key"] == "t_col_w1_0")
        self.assertEqual(stations(flank, TEMPLE), [14400 + 1800])
        self.assertEqual(sorted(i["key"] for i in out["instances"] if i["key"].startswith("t_col_c")),
                         ["t_col_c0", "t_col_c1"])
        self.assertEqual(len([i for i in out["instances"] if i["key"].startswith("t_platform_f")]), 14 * 2)

    def test_a_side_that_is_not_whole_bays_is_said_of_the_colonnade(self):
        ring = [(0, 0), (14400, 0), (14400, 20300), (0, 20300)]
        out = lay(ring=ring)
        said = sorted((o["side"], o["length"], o["near"]) for o in out["open"] if o.get("colonnade"))
        self.assertEqual(said, [(1, 18500, (18000.0, 20700.0)), (3, 18500, (18000.0, 20700.0))])
        self.assertTrue(all(o["posts"] for o in out["open"] if o.get("colonnade")))

    def test_a_kit_with_no_colonnade_pieces_leaves_the_platform_open_and_says_so(self):
        rows = [r for r in order() if r["id"] not in ("column", "lintel")]
        out = lay(rows)
        self.assertEqual([n["code"] for n in out["notes"]], ["colonnade_none"])
        self.assertEqual(out["storeys"][0]["region"], CELLA)
        self.assertFalse([r for r in out["runs"] if r["key"].startswith("t_col_")])

    def test_what_a_colonnade_is_refused_for(self):
        for attrs, ring, text in (
                ({"storeys": 2, "stair": {"side": 2, "at": 3, "width": 1, "run": 5}}, TEMPLE, "one storey"),
                ({}, [(0, 0), (14400, 0), (14400, 9000), (9000, 9000), (9000, 19800), (0, 19800)], "a rectangle only"),
                ({"colonnade": {"sides": "all", "depth": 2.7}}, TEMPLE, "a list of tables"),
                ({"colonnade": [{"sides": "all"}]}, TEMPLE, "is not `{ sides, depth }`"),
                ({"colonnade": [{"sides": [4], "depth": 2.7}]}, TEMPLE, "is not `{ sides, depth }`"),
                ({"colonnade": [{"sides": [0], "depth": 2.7, "post": "x"}]}, TEMPLE, "is not `{ sides, depth }`"),
                ({"colonnade": [{"sides": [0], "depth": 2.7}, {"sides": "all", "depth": 2.7}]}, TEMPLE, "twice"),
                ({"colonnade": [{"sides": [1, 3], "depth": 6.8}]}, TEMPLE, "less than a metre deep")):
            with self.subTest(text=text):
                with self.assertRaises(PE) as e:
                    lay(ring=ring, **attrs)
                self.assertEqual(e.exception.key, "colonnade")
                self.assertIn(text, e.exception.message)

    def test_a_building_with_no_colonnade_is_laid_as_before(self):
        out = lay(colonnade=[])
        self.assertIsNone(out["platform"])
        self.assertEqual(out["storeys"][0]["region"], TEMPLE)
        self.assertEqual([d["key"] for d in out["doorways"]], ["t_d0"])
        self.assertFalse([r for r in out["runs"] if r["key"].startswith("t_col_")])
        self.assertEqual({r["ref"] for r in out["runs"] if r["key"].startswith("t_base_")}, {"t_s0"})

    def test_a_hole_in_a_floor_takes_no_tile_and_a_cell_partly_in_it_is_short(self):
        tile = {"id": "tile", "pitch": 900}
        ring = [(0, 0), (5400, 0), (5400, 5400), (0, 5400)]
        hole = [(1800, 1800), (3600, 1800), (3600, 3600), (1800, 3600)]
        insts, (short, _) = L["slab_plan"](ring, ring, 0, tile, "p", "p.floor", hole)
        self.assertEqual((len(insts), short), (36 - 4, 0))
        off = [(1800, 1800), (4000, 1800), (4000, 3600), (1800, 3600)]
        insts, (short, first) = L["slab_plan"](ring, ring, 0, tile, "p", "p.floor", off)
        self.assertEqual((len(insts), short, first), (36 - 6, 2, (4050, 2250)))


class Mirror(unittest.TestCase):
    """lay.py rule 8: a `fill` read from both ends of a side toward its middle."""

    def words(self, n, mirror, pattern=("wall", "niche"), first=900, side_len=None, pitch=1800):
        picks = L["picks_of"]({"fill": [{"pattern": list(pattern), "mirror": mirror}]}, 1, 4)
        side_len = side_len if side_len is not None else 2 * first + n * pitch
        return L["bay_variants"](n, first, pitch, 0, 0, picks, "t", None, side_len, 900)

    def test_a_pattern_read_from_the_start_is_a_translation(self):
        self.assertEqual(self.words(4, False), [None, "niche", None, "niche"])

    def test_a_mirrored_pattern_is_read_from_both_ends(self):
        self.assertEqual(self.words(4, True), [None, "niche", "niche", None])
        self.assertEqual(self.words(7, True), [None, "niche", None, "niche", None, "niche", None])
        self.assertEqual(self.words(5, True, ("a", "b", "c")), ["a", "b", "c", "b", "a"])

    def test_the_two_walls_beside_a_door_on_the_axis_are_images(self):
        """A side of 9 bays with its middle bay a door: the stretch after the door is counted
        from the side's far end, not from its own start."""
        side = 2 * 900 + 9 * 1800
        left = self.words(4, True, ("a", "b", "c"), 900, side)
        right = self.words(4, True, ("a", "b", "c"), 900 + 5 * 1800, side)
        self.assertEqual((left, right), (["a", "b", "c", "a"], ["a", "c", "b", "a"]))
        plain = (self.words(4, False, ("a", "b", "c"), 900, side), self.words(4, False, ("a", "b", "c"), 900 + 5 * 1800, side))
        self.assertEqual(plain, (["a", "b", "c", "a"], ["a", "b", "c", "a"]))

    def test_a_choice_in_a_mirrored_fill_is_the_same_in_both_halves(self):
        for name in ("t", "u", "v", "w"):
            picks = L["picks_of"]({"fill": [{"pattern": [["a", "b", "c"]], "mirror": True}]}, 1, 4)
            got = L["bay_variants"](8, 900, 1800, 0, 0, picks, name, None, 2 * 900 + 8 * 1800, 900)
            self.assertEqual(got, got[::-1], name)
        picks = L["picks_of"]({"fill": [{"pattern": [["a", "b"]], "mirror": True}]}, 1, 4)
        got = L["bay_variants"](6, 900, 1800, 0, 0, picks, "t", None, 2 * 900 + 6 * 1800, 900)
        self.assertEqual(got, got[::-1])
        self.assertEqual(len(set(got[:3])), 2)           # two pieces still go in turn, from each end

    def test_the_temples_back_wall(self):
        out = lay(fill=[{"sides": [1, 2, 3], "pattern": ["wall", "niche"], "mirror": True}])
        back = next(r for r in out["runs"] if r["key"] == "t_s0_w2_0")
        self.assertEqual(back["assets"], ["wall", "wall_niche", "wall_niche", "wall"])
        out = lay(fill=[{"sides": [1, 2, 3], "pattern": ["wall", "niche"]}])
        back = next(r for r in out["runs"] if r["key"] == "t_s0_w2_0")
        self.assertEqual(back["assets"], ["wall", "wall_niche", "wall", "wall_niche"])

    def test_mirror_is_true_or_false(self):
        with self.assertRaises(PE) as e:
            L["picks_of"]({"fill": [{"pattern": ["wall"], "mirror": "yes"}]}, 1, 4)
        self.assertIn("`mirror` is true or false", e.exception.hint)
        self.assertFalse(L["picks_of"]({"fill": [{"pattern": ["wall"]}]}, 1, 4)["fill"][0]["mirror"])


class FakeOut:
    def __init__(self):
        self.features, self.instances, self.tables = {}, {}, {}

    def feature(self, key, kind, at=None, path=None, region=None, **attrs):
        self.features[key] = dict(attrs, kind=kind, path=path, region=region)

    def instance(self, key, asset, at, face=None, **keys):
        self.instances[key] = dict(keys, asset=asset, at=at, face=face)

    def table(self, section, key, **keys):
        self.tables[(section, key)] = keys


class _FL(dict):
    def __len__(self):
        return len(self["geometry"])

    def rings(self, f):
        return [[self["coords"]]]


class FakeStep:
    """The step as `lay` reads it: one building, level ground, the order kit's rows."""

    def __init__(self, ring=TEMPLE, rows=None, **attrs):
        self.params, self.grid, self.id, self.for_each_item = {}, None, "lay", "t"
        self.out, self.attrs, self.ring, self.said = FakeOut(), {**ATTRS, **attrs}, ring, []
        self.rows = order() if rows is None else rows

    def features(self, select, columns=()):
        fl = _FL({"name": ["t"], "geometry": [2], "coords": [list(p) for p in self.ring]})
        for c in columns:
            v = self.attrs.get(c)
            fl[c] = [json.dumps(v) if isinstance(v, (list, dict)) else (v if c in self.attrs else 0)]
            fl[c + "?"] = [1 if c in self.attrs else 0]
        return fl

    def fields(self, names, **kw):
        class A(list):
            name = "height"
        return [A()]

    def eval(self, formula, x, y):
        return [10.0] * len(x)

    def pack(self, select, columns=()):
        return {"id": [r["id"] for r in self.rows], "tags": [json.dumps(r["tags"]) for r in self.rows],
                "bounds": [r["bounds"] for r in self.rows], "params": [json.dumps(r["params"]) for r in self.rows],
                "shared": [json.dumps(r["shared"]) for r in self.rows]}

    def _say(sev):
        def say(self, code, subject, message, hint, **kw):
            self.said.append((sev, code, message, hint, kw.get("data")))
        return say
    error, warning, info = _say("error"), _say("warning"), _say("info")


class Said(unittest.TestCase):
    """What `lay` writes and says of a temple (the step, through a fake of the SDK)."""

    def test_the_temple_is_written_with_its_platform_and_nothing_is_said(self):
        s = FakeStep()
        L["generate"](s)
        self.assertEqual([x[:2] for x in s.said], [("info", "laid")])
        f = s.out.features
        self.assertEqual(sorted(f), ["t_d0", "t_d0_in", "t_p0", "t_platform", "t_roof", "t_s0"])
        self.assertEqual((f["t_platform"]["kind"], f["t_platform"]["level"], f["t_platform"]["region"]), ("platform", 10.9, TEMPLE))
        self.assertEqual(f["t_d0"]["ends_at"], "t_platform.floor")
        run = s.out.tables[("run", "t_col_p0")]
        self.assertEqual((run["asset"], run["count"], run["on"]["ref"], run["on"]["offset"]), ("column", 4, "t_platform", 0.45))
        self.assertEqual(sum(1 for i in s.out.instances.values() if i["surface"] == "t_platform.floor"),
                         4 + 4 + 14 * 20 - 10 * 16)          # corner columns, base corners, the platform's tiles

    def test_a_side_off_the_bay_is_said_of_the_colonnade_with_what_would_close(self):
        s = FakeStep(ring=[(0, 0), (14400, 0), (14400, 20300), (0, 20300)])
        L["generate"](s)
        said = [x for x in s.said if x[1] == "wall_open" and "of the colonnade" in x[2]]
        self.assertEqual(len(said), 2)
        self.assertIn("a stretch of 18.5 m on side 1 of the colonnade is no whole number of bays: a bay is 2.7 m "
                      "from one post's middle to the next, and a post 0.9 m wide", said[0][2])
        self.assertIn("resize the side by -0.5 m or +2.2 m", said[0][3])
        self.assertEqual(said[0][4]["closes"], [18.0, 20.7])
        platform = [x for x in s.said if x[1] == "slab_open" and "of the platform" in x[2]]
        self.assertEqual(len(platform), 1)
        # the flank is 20.3 m: 18.5 m inside the base, which is what is off the tiles, not the depth
        self.assertIn("side 1 is 18.5 m long inside the line its tiles start on, no whole number of 0.9 m tiles",
                      platform[0][2])
        self.assertIn("make that side 19.8 or 20.7 m long", platform[0][3])

    def test_a_kit_with_no_colonnade_is_said(self):
        s = FakeStep(rows=[r for r in order() if r["id"] not in ("column", "lintel")])
        L["generate"](s)
        said = [x for x in s.said if x[0] == "warning"]
        self.assertEqual([x[1] for x in said], ["colonnade_none"])
        self.assertIn("both tagged `storey:colonnade`", said[0][3])

    def test_a_column_in_the_doorway_stops_the_building(self):
        s = FakeStep(ring=[(0, 0), (11700, 0), (11700, 19800), (0, 19800)], doors=[{"storey": 0, "side": 0, "at": 5.85}])
        L["generate"](s)
        self.assertEqual([(x[0], x[1], x[4]) for x in s.said], [("error", "params", {"attribute": "doors"})])
        # an even front: every bay's middle is off the wall's, which takes the frame from 5.4 to 6.3 m
        self.assertIn("its middle is at 1.8, 4.5, 7.2, 9.9 m along the side, and the wall behind takes it from 5.4 "
                      "to 6.3 m", s.said[0][3])
        self.assertEqual(s.out.features, {})


def judged(rows, features=(), meetings=(), pieces=()):
    return [(r[0], r[1], r[2], r[3]) for r in C["judge"](list(features), list(meetings), rows, P, list(pieces))
            if r[1] != "checked"]


class Checked(unittest.TestCase):
    """check.py: what a kit of posts, and a kit that states a unit, is held to."""

    def test_the_temples_kit_says_nothing(self):
        self.assertEqual(judged(order()), [])

    def test_a_panel_is_its_bay_less_one_post(self):
        said = judged(order(lintel=2700))
        self.assertEqual([(s[1], s[2]) for s in said], [("kit_pitch", "lintel")])
        self.assertIn("`lintel` is 2.7 m wide, and its bay of 2.7 m leaves 1.8 m between two posts 0.9 m wide", said[0][3])
        # a piece of another group is held to its own pitch, as before
        self.assertEqual([s[1] for s in judged(order(swap={"base": row("base", "wall", (1700, 900), 900, ("storey:base",), pitch=1.8)}))],
                         ["kit_pitch", "kit_unit"])

    def test_a_column_re_exported_wider_is_said_of_the_column_and_of_its_lintel(self):
        said = judged(order(column=1000))
        self.assertEqual(sorted((s[1], s[2]) for s in said), [("kit_pitch", "lintel"), ("kit_unit", "column")])
        unit = next(s for s in said if s[1] == "kit_unit")
        self.assertIn("a measured length of 1 m and a measured thickness of 1 m, no whole number of the kit's unit (0.9 m)",
                      unit[3])
        # and it is not told it should be the wall's thickness square: a corner post takes its own width
        self.assertFalse([s for s in said if s[1] == "kit_corner"])

    def test_every_size_a_piece_states_is_held_to_the_unit(self):
        rows = order(swap={"roof_slope": row("roof_slope", "roof_slope", (1800, 7000), 2700, pitch=1.8, span=7.0)})
        said = [s for s in judged(rows) if s[1] == "kit_unit"]
        self.assertEqual(len(said), 1)
        self.assertIn("`roof_slope` has a `params.span` of 7 m", said[0][3])

    def test_what_the_kit_states_is_held_to_its_unit_too(self):
        rows = [dict(r, shared=dict(SHARED, door=2.0, storey=[9.0, 4.0])) for r in order()]
        said = sorted(s[3] for s in judged(rows) if s[1] == "kit_unit")
        self.assertEqual(len(said), 2)
        self.assertIn("states `shared.door = 2`", said[0])
        self.assertIn("states a storey of 4 m", said[1])

    def test_a_kit_with_no_unit_is_not_held_to_one(self):
        rows = [dict(r, shared={k: v for k, v in SHARED.items() if k != "unit"}) for r in order(column=1000)]
        self.assertFalse([s for s in judged(rows) if s[1] == "kit_unit"])

    def test_a_colonnades_piece_is_held_to_the_storey_by_its_top(self):
        """The lintel's box is 1.8 m high and starts 7.2 m up: it stands a storey high."""
        self.assertFalse([s for s in judged(order()) if s[1] == "kit_stated"])
        low = row("lintel", "wall", (1800, 900), 1800, ("storey:colonnade",), z0=6300, pitch=2.7)
        said = [s for s in judged(order(swap={"lintel": low})) if s[1] == "kit_stated"]
        self.assertEqual(len(said), 1)
        self.assertIn("`lintel` stands 8.1 m high where the kit states a storey of 9 m", said[0][3])

    def test_a_lintel_over_a_doorway_is_not_in_its_way(self):
        path = [(7200, -2000), (7200, 0)]
        lintel = {"name": "lintel", "x": 7200, "y": 450, "z": 10.9, "face": 180000, "scale": 1.0,
                  "bounds": [-900, -450, 7200, 900, 450, 9000]}
        cols = [{"name": f"col{k}", "x": x, "y": 450, "z": 10.9, "face": 180000, "scale": 1.0,
                 "bounds": [-450, -450, 0, 450, 450, 9000]} for k, x in enumerate((5850, 8550))]
        clear, beside = C["clear_width"](path, [lintel] + cols, 10.9)
        self.assertEqual((clear, sorted(beside)), (1800.0, ["col0", "col1"]))
        on_floor = dict(lintel, bounds=[-900, -450, 0, 900, 450, 1800])
        self.assertEqual(C["clear_width"](path, [on_floor] + cols, 10.9)[0], 0.0)

    def test_the_room_behind_a_platform_is_reached_through_the_platform(self):
        out = lay()
        feats = [{"name": "lay#t_s0", "kind": "storey", "ring": CELLA, "path": CELLA, "attrs": {"level": 10.9}},
                 {"name": "lay#t_platform", "kind": "platform", "ring": TEMPLE, "path": TEMPLE, "attrs": {"level": 10.9}}]
        for d in out["doorways"]:
            feats.append({"name": "lay#" + d["key"], "kind": "doorway", "ring": d["path"], "path": d["path"],
                          "attrs": {"sill_z": d["sill_z"]}})

        def meet(a, b):
            return {"a": a, "b": b, "gap": 0, "connector_a": "foot", "connector_b": None, "x": 0, "y": 0, "turn": None}
        both = [meet("lay#t_d0.end", "lay#t_platform.floor"), meet("lay#t_d0_in.end", "lay#t_s0.floor")]
        self.assertFalse([s for s in judged([], feats, both) if s[1] == "unreached"])
        # with no way onto the platform, the room behind it is not reached: the second doorway
        # starts on the platform, not on the terrain
        said = [s for s in judged([], feats, both[1:]) if s[1] == "unreached"]
        self.assertEqual([s[2] for s in said], ["lay#t_s0"])


if __name__ == "__main__":
    unittest.main()
