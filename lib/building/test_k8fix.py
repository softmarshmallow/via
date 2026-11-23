"""Tests from K8's one independent review (docs/plan-kits.md): what it found in lay.py rules 8
and 15 to 18, court.py and check.py, each held here by the case that showed it.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The kits are the two examples', as test_posts.py (`order`, examples/temple) and test_court.py
(`timber` and `yard`, examples/court) give their rows.
"""

import os
import runpy
import types
import unittest

# The two examples' test modules, for their kits and fakes. Run, not imported: library code
# imports only viasdk and the standard library (R19b), and its tests keep to that.
HERE = os.path.dirname(os.path.abspath(__file__))
T = types.SimpleNamespace(**runpy.run_path(os.path.join(HERE, "test_court.py")))
O = types.SimpleNamespace(**runpy.run_path(os.path.join(HERE, "test_posts.py")))

L, C, K = T.L, T.C, T.K
PE, KE = (L["ParamError"], O.L["ParamError"]), K["ParamError"]   # each test module runs lay.py for itself


def runs_of(out, start):
    return {r["key"]: r for r in out["runs"] if r["key"].startswith(start)}


class StairBesidePosts(unittest.TestCase):
    """Rule 3 with rule 15: a flight climbs beside a wall of posts and crosses nothing. The
    wall's band is the post's width; before, the flight was set off the panel's thickness and
    the crossing read off the post's, so the stair's side lost every panel it ran along."""

    RING = [(0, 0), (12300, 0), (12300, 9300), (0, 9300)]            # four bays by three
    STAIR = {"side": 2, "at": 3.15, "width": 1.0, "run": 4.0}

    def lay(self, **attrs):
        return T.lay(self.RING, storeys=2, roof="flat", stair=self.STAIR, **attrs)

    def test_every_panel_and_post_of_the_stairs_side_stands_on_both_storeys(self):
        out = self.lay()
        for k in (0, 1):
            panels = [r for r in out["runs"] if r["key"].startswith(f"t_s{k}_w2_")]
            posts = [r for r in out["runs"] if r["key"].startswith(f"t_s{k}_p2")]
            self.assertEqual(sum(r["count"] for r in panels), 4, k)
            self.assertEqual(sum(r["count"] for r in posts), 3, k)
        self.assertEqual(out["open"], [])

    def test_the_flight_keeps_off_the_posts_not_off_the_panels(self):
        out = self.lay()
        (flight,) = out["flights"]
        # side 2 runs east to west along y = 9.3 m: the post's 0.3 m, the clear 0.1 m, half the width
        self.assertEqual({p[1] for p in flight["path"]}, {9300 - 300 - 100 - 500})

    def test_the_footing_over_the_stairwell_is_as_deep_as_a_post(self):
        out = self.lay()
        (footing,) = out["footings"]
        self.assertEqual(sorted({p[1] for p in footing["region"]}), [9000, 9300])
        upper = [r for r in out["runs"] if r["key"].startswith("t_s1_w2_") or r["key"].startswith("t_s1_p2")]
        self.assertEqual({r["ref"] for r in upper}, {"t_s1_footing"})

    def test_the_stairwell_keeps_a_posts_width_off_the_corner(self):
        """A stair from the side's start: the stairwell is held off the wall beside it by the
        band, which is the corner post's 0.3 m, not the panel's 0.15 m."""
        out = T.lay(self.RING, storeys=2, roof="flat", stair={"side": 2, "at": 0.5, "width": 1.0, "run": 4.0})
        xs = sorted({p[0] for p in out["storeys"][1]["region"]})
        self.assertEqual(xs[-2:], [12000, 12300])

    def test_what_is_said_of_a_stairwell_off_the_tiles_is_measured_from_the_post_line(self):
        """A kit whose panel is as thick as its post: the tiles still start half a post in."""
        rows = [dict(r) for r in O.PostWalls.ROWS] + [O.row("tile", "floor", (1500, 1500), 100, shared={}, pitch=1.5)]
        out = T.lay(self.RING, rows=rows, storeys=2, storey_h=3.0, roof="flat", stair=self.STAIR)
        (note,) = [n for n in out["notes"] if n["code"] == "slab_open" and n["storey"] == 1]
        # the stairwell is the post's 0.3 m, the flight's 1 m and 0.1 m each side deep: 1.35 m of tiles
        self.assertEqual((note["deep"], note["from"], note["to"], note["width"]), (1350.0, 2000.0, 7000.0, 1150.0))

    def test_the_floor_over_the_footing_is_still_tiled_to_the_post_line(self):
        """The upper floor's last row of tiles lies half a post onto the footing; only the four
        cells the stairwell cuts are left open."""
        out = self.lay()
        tiles = [i for i in out["instances"] if i["key"].startswith("t_s1_f")]
        self.assertEqual(len(tiles), 44)
        self.assertEqual(max(i["at"][1] for i in tiles), 8400)
        (note,) = [n for n in out["notes"] if n["code"] == "slab_open"]
        self.assertEqual((note["storey"], note["cells"]), (1, 4))

    def test_a_kit_of_whole_wall_pieces_keeps_its_wall_as_the_band(self):
        """`examples/kit`'s arithmetic is untouched: a 0.3 m wall, lane 0 at 0.3 + 0.1 + half."""
        sp = L["stair_plan"]([(0, 0), (12000, 0), (12000, 9000), (0, 9000)], {"side": 2, "at": 3, "width": 1.0, "run": 4.0},
                             2, 300)
        self.assertEqual(sp["flights"][0][0][0][1], 9000 - 300 - 100 - 500)


class DoorBehindColonnade(unittest.TestCase):
    """Rule 16: a door through a colonnade is said in the measure its `at` was written in, the
    building's side, and every place it is offered takes the door."""

    def refused(self, lay, **attrs):
        with self.assertRaises(PE) as e:
            lay(**attrs)
        self.assertEqual(e.exception.key, "doors")
        return e.exception

    def places(self, hint):
        return [float(v) for v in hint.split("two posts: ")[1].split(" m along")[0].split(", ")]

    def test_the_temples_door_is_said_along_the_front_and_its_one_place_is_taken(self):
        e = self.refused(O.lay, doors=[{"storey": 0, "side": 0, "at": 9.9}])
        self.assertIn("door 0 is 9.9 m along side 0", e.message)
        self.assertEqual(self.places(e.hint), [7.2])
        O.lay(doors=[{"storey": 0, "side": 0, "at": 7.2}])
        # the reviewer's second try, inside the range the old message gave: still said along the front
        e = self.refused(O.lay, doors=[{"storey": 0, "side": 0, "at": 5.0}])
        self.assertIn("door 0 is 5 m along side 0", e.message)
        self.assertEqual(self.places(e.hint), [7.2])

    def test_behind_a_colonnade_on_two_sides_every_middle_is_offered_and_each_is_taken(self):
        """Timber, a veranda on the front and the left flank: the wall behind the front starts
        3 m along it, so its bays' middles are 4.65 to 13.65 m along the building's side."""
        col = [{"sides": [0, 3], "depth": 3.0}]
        e = self.refused(T.lay, colonnade=col, doors=[{"storey": 0, "side": 0, "at": 7.3}])
        self.assertIn("door 0 is 7.3 m along side 0", e.message)
        self.assertIn("off the middle of its bay in the wall behind the colonnade", e.message)
        self.assertEqual(self.places(e.hint), [4.65, 7.65, 10.65, 13.65])
        for at in self.places(e.hint):
            out = T.lay(colonnade=col, doors=[{"storey": 0, "side": 0, "at": at}])
            self.assertEqual([i["key"] for i in out["instances"] if i["key"].endswith("_door0")], ["t_s0_door0"])
        # 1.65 m is a bay of the colonnade's, beside the room: no wall stands behind it
        e = self.refused(T.lay, colonnade=col, doors=[{"storey": 0, "side": 0, "at": 1.65}])
        self.assertIn("the wall behind the colonnade stands from 3 to 15.3 m", e.message)

    def test_a_door_too_wide_for_a_bay_of_the_wall_behind(self):
        e = self.refused(T.lay, colonnade=[{"sides": [0], "depth": 3.0}],
                         doors=[{"storey": 0, "side": 0, "at": 7.65, "width": 2.8}])
        self.assertIn("a post of the colonnade stands 6.15 m along it", e.message)
        self.assertIn("the doorway is 2.8 m wide, and the colonnade leaves 2.7 m clear between two posts", e.hint)

    def test_a_door_too_wide_to_pass_between_two_columns_is_offered_no_place(self):
        """The wall behind takes a 2 m doorway on its middle; the columns, 1.8 m apart, do not."""
        e = self.refused(O.lay, doors=[{"storey": 0, "side": 0, "at": 7.2, "width": 2.0}])
        self.assertIn("a post of the colonnade stands 5.85 m along it, in the doorway", e.message)
        self.assertEqual(e.hint, "the doorway is 2 m wide, and the colonnade leaves 1.8 m clear between two posts: "
                         "write a `width` of that or less")
        O.lay(doors=[{"storey": 0, "side": 0, "at": 7.2, "width": 1.8}])

    def test_with_no_colonnade_pieces_the_walls_middles_are_still_the_buildings(self):
        rows = T.timber(drop=("veranda_post", "beam"))
        e = self.refused(T.lay, rows=rows, colonnade=[{"sides": [0, 3], "depth": 3.0}],
                         doors=[{"storey": 0, "side": 0, "at": 7.3}])
        self.assertEqual(self.places(e.hint), [4.65, 7.65, 10.65, 13.65])
        self.assertNotIn("where the wall behind takes it too", e.hint)

    def test_an_even_front_has_no_place_and_says_both_reasons(self):
        ring = [(0, 0), (11700, 0), (11700, 19800), (0, 19800)]
        e = self.refused(O.lay, ring=ring, doors=[{"storey": 0, "side": 0, "at": 5.85}])
        self.assertIn("a post of the colonnade stands 5.85 m along it, in the doorway", e.message)
        self.assertEqual(e.hint, "no place on this side takes the door: between two posts of the colonnade its middle "
                         "is at 1.8, 4.5, 7.2, 9.9 m along the side, and the wall behind takes it from 5.4 to 6.3 m. "
                         "Change the number of bays or the door's width, or use another side")
        for at in (1.8, 4.5, 7.2, 9.9):
            self.refused(O.lay, ring=ring, doors=[{"storey": 0, "side": 0, "at": at}])

    def test_a_door_on_a_side_the_walls_keep_is_said_along_the_buildings_side(self):
        """A veranda on the front only: the flank's wall starts 2.7 m along the flank. The frame
        is 3.6 m, and the wall's corners take 0.9 m each."""
        col = [{"sides": [0], "depth": 2.7}]
        e = self.refused(O.lay, colonnade=col, doors=[{"storey": 0, "side": 1, "at": 4.0}])
        self.assertIn("door 0 is 4 m along side 1", e.message)
        lo, hi = [float(v) for v in e.hint.split("between ")[1].split(" m along")[0].split(" and ")]
        self.assertEqual((lo, hi), (5.4, 17.1))
        for at in (lo, hi):
            O.lay(colonnade=col, doors=[{"storey": 0, "side": 1, "at": at}])
        self.refused(O.lay, colonnade=col, doors=[{"storey": 0, "side": 1, "at": lo - 0.1}])
        self.refused(O.lay, colonnade=col, doors=[{"storey": 0, "side": 1, "at": hi + 0.1}])

    def test_a_door_off_a_bay_on_a_kept_side_of_posts_is_offered_the_buildings_middles(self):
        col = [{"sides": [0], "depth": 3.0}]
        e = self.refused(T.lay, colonnade=col, doors=[{"storey": 0, "side": 1, "at": 5.0}])
        self.assertIn("door 0 is 5 m along side 1", e.message)
        mids = [float(v) for v in e.hint.split("of a bay: ")[1].split(" m along")[0].split(", ")]
        self.assertEqual(mids, [4.65, 7.65])
        for at in mids:
            T.lay(colonnade=col, doors=[{"storey": 0, "side": 1, "at": at}])


def said_of(step, code):
    return [x for x in step.said if x[1] == code]


def timber_step(ring=T.HALL, rows=None, **attrs):
    """`lay` run as a step on one timber building; what it said is `(severity, code, subject,
    message, hint, data)`."""
    step = T.FakeStep({"t": (ring, {**T.ATTRS, **attrs})}, "t")
    if rows is not None:
        step.pack = lambda select, columns=(): {
            "id": [r["id"] for r in rows], "tags": [T.json.dumps(r["tags"]) for r in rows],
            "bounds": [r["bounds"] for r in rows], "params": [T.json.dumps(r["params"]) for r in rows],
            "shared": [T.json.dumps(r["shared"]) for r in rows]}
    L["generate"](step)
    return step


class GableUnderEaves(unittest.TestCase):
    """Rule 17: of several gable halves the one made for the slope's span less the eaves is
    taken. Before, a gable was matched to the slope's own span, so a kit that had the right half
    was told it had not."""

    HALVES = [T.row("gable_315", "roof_gable", (3150, 150), 1800, span=3.15),
              T.row("gable_465", "roof_gable", (4650, 150), 2400, span=4.65)]

    def rows(self):
        return T.timber() + self.HALVES

    def roofed(self, ring, slope, half, span):
        out = T.lay(ring, rows=self.rows(), roof="gable")
        self.assertEqual(T.notes(out, "roof_none"), [])
        runs, insts = T.roof_of(out)
        self.assertEqual({r["asset"] for r in runs.values()}, {slope})
        self.assertEqual({i["asset"] for i in insts.values()}, {half})
        # covered once: two runs of slopes, each piece its pitch along the eave and its span deep
        (x0, y0), (x1, y1) = out["roof"]["region"][0], out["roof"]["region"][2]
        self.assertEqual(sum(r["count"] for r in runs.values()) * 3000 * span, (x1 - x0) * (y1 - y0))
        self.assertEqual(min(y1 - y0, x1 - x0), 2 * span)
        return insts

    def test_a_hall_three_bays_deep_takes_the_larger_half_and_is_covered(self):
        insts = self.roofed(T.HALL, "roof_slope_6", "gable_465", 6000)
        # each half stands the eaves in from the roof's edge, over the end wall: 9.3 m in two halves
        self.assertEqual(sorted(i["at"] for k, i in insts.items() if k.startswith("t_roof_g1")),
                         [(15225, 2325), (15225, 6975)])

    def test_a_wing_two_bays_deep_takes_the_smaller(self):
        self.roofed(T.WING, "roof_slope_45", "gable_315", 4500)

    def test_the_kit_with_both_halves_says_nothing_of_them(self):
        self.assertEqual([r for r in O.judged(self.rows()) if "gable" in str(r)], [])

    def test_with_one_slope_the_half_is_still_chosen_by_the_span_less_the_eaves(self):
        rows = T.timber(drop=("roof_slope_45", "roof_hip_45")) + [
            T.row("gable_6", "roof_gable", (6000, 150), 2400, span=6.0)] + self.HALVES
        out = T.lay(T.HALL, rows=rows, roof="gable")
        self.assertEqual(T.notes(out, "roof_none"), [])
        self.assertEqual({i["asset"] for i in T.roof_of(out)[1].values()}, {"gable_465"})

    def test_with_no_eaves_the_half_of_the_slopes_own_span_is_taken(self):
        plain = {k: v for k, v in T.SHARED.items() if k != "eaves"}
        rows = [dict(r, shared=plain) for r in T.timber() + [T.row("gable_6", "roof_gable", (6000, 150), 2400, span=6.0),
                                                           T.row("gable_45", "roof_gable", (4500, 150), 1800, span=4.5)]]
        out = T.lay([(0, 0), (15300, 0), (15300, 9000), (0, 9000)], rows=rows, roof="gable")
        self.assertEqual(T.notes(out, "roof_none"), [])
        self.assertEqual({i["asset"] for i in T.roof_of(out)[1].values()}, {"gable_45"})


class ColonnadeEnd(unittest.TestCase):
    """Rule 16: the colonnade's end on a side the walls keep is as long as the colonnade is
    deep, so `depth` is what closes it."""

    def test_an_end_that_does_not_close_is_said_of_the_depth(self):
        step = timber_step(colonnade=[{"sides": [0], "depth": 2.5}])
        ends = [x for x in said_of(step, "wall_open") if "the colonnade's end" in x[3]]
        self.assertEqual(len(ends), 2)
        self.assertIn("the colonnade's end on side 1, 2.5 m from the building's corner to the wall, is no whole "
                      "number of bays", ends[0][3])
        self.assertEqual(ends[0][4], "make the colonnade's `depth` 3 m: its end beside the room is then whole bays")
        self.assertEqual(ends[0][5]["closes"], [3.0])
        self.assertTrue(ends[0][5]["depth"])
        self.assertEqual(ends[0][5]["length"], 2.5)
        # the depth it names closes both ends
        again = timber_step(colonnade=[{"sides": [0], "depth": 3.0}])
        self.assertEqual([x for x in said_of(again, "wall_open") if "colonnade" in x[3]], [])

    def test_a_depth_between_two_bays_is_offered_both(self):
        step = timber_step(colonnade=[{"sides": [0], "depth": 4.0}])
        (end,) = [x for x in said_of(step, "wall_open") if "the colonnade's end on side 1" in x[3]]
        self.assertIn("make the colonnade's `depth` 3 or 6 m", end[4])

    def test_a_whole_side_of_the_colonnade_is_still_said_of_the_side(self):
        step = timber_step(ring=[(0, 0), (15800, 0), (15800, 9300), (0, 9300)], colonnade=[{"sides": [0], "depth": 3.0}],
                           doors=[])
        (side,) = [x for x in said_of(step, "wall_open") if "of the colonnade" in x[3]]
        self.assertIn("a stretch of 15.2 m on side 0 of the colonnade is no whole number of bays", side[3])
        self.assertIn("resize the side by", side[4])
        self.assertNotIn("depth", side[5])

    def test_a_depth_thinner_than_a_post_is_refused_as_that(self):
        with self.assertRaises(PE) as e:
            T.lay(colonnade=[{"sides": [0], "depth": 0.2}])
        self.assertEqual(e.exception.key, "colonnade")
        self.assertIn("`depth` is 0.2 m, less than its post is wide (0.3 m)", e.exception.message)
        self.assertIn("3 m", e.exception.hint)
        # as deep as a post is no longer too thin (what does not close is then said of the depth)
        T.lay(colonnade=[{"sides": [0], "depth": 0.3}])

    def test_with_no_colonnade_pieces_a_thin_depth_is_only_a_set_back_wall(self):
        out = T.lay(rows=T.timber(drop=("veranda_post", "beam")), colonnade=[{"sides": [0], "depth": 0.2}])
        self.assertEqual([n["code"] for n in out["notes"] if n["code"] == "colonnade_none"], ["colonnade_none"])


class PlatformTiles(unittest.TestCase):
    """Rule 16: what `slab_open` says of the platform is true, and the depth it offers lays whole
    tiles."""

    def platform(self, lay, **attrs):
        out = lay(**attrs)
        return out, [n for n in out["notes"] if n["code"] == "slab_open" and n.get("platform")]

    def test_a_veranda_one_bay_deep_is_whole_tiles_with_no_base(self):
        """Before, a kit with no base had the platform's tiles start a post's width in, and a
        veranda one bay deep left 11 of them unlaid."""
        rows = T.timber(drop=("base", "base_corner"))
        out, open_ = self.platform(T.lay, rows=rows, colonnade=[{"sides": [0], "depth": 3.0}])
        self.assertEqual(open_, [])
        tiles = [i for i in out["instances"] if i["key"].startswith("t_platform_f")]
        self.assertEqual(len(tiles), 2 * 10)                             # 15 m by 3 m of 1.5 m tiles
        self.assertEqual(min(i["at"][1] for i in tiles), 150 + 750)      # from the middle of the line of posts

    def test_the_depth_offered_behind_posts_lays_whole(self):
        for rows in (T.timber(), T.timber(drop=("base", "base_corner"))):
            _, open_ = self.platform(T.lay, rows=rows, colonnade=[{"sides": [0], "depth": 3.3}])
            (n,) = open_
            self.assertEqual([(d["side"], d["deep"], d["closes"]) for d in n["deep"]], [(0, 3300.0, [3000.0, 4500.0])])
            self.assertEqual(n["to"], "the middle of the wall's line of posts")
            for depth in n["deep"][0]["closes"]:
                self.assertEqual(self.platform(T.lay, rows=rows, colonnade=[{"sides": [0], "depth": depth / 1000}])[1], [])
        # the reviewer's other depth: 1.8 m is off the tiles, 1.5 m on them
        self.assertEqual(self.platform(T.lay, colonnade=[{"sides": [0], "depth": 1.5}])[1], [])
        self.assertEqual(len(self.platform(T.lay, colonnade=[{"sides": [0], "depth": 1.8}])[1]), 1)

    def test_the_depth_offered_before_a_wall_of_whole_pieces_lays_whole(self):
        _, open_ = self.platform(O.lay, colonnade=[{"sides": "all", "depth": 3.0}], doors=[])
        (n,) = open_
        self.assertEqual(n["to"], "the wall")
        self.assertEqual(n["deep"][0]["closes"], [2700.0, 3600.0])
        for depth in n["deep"][0]["closes"]:
            self.assertEqual(self.platform(O.lay, colonnade=[{"sides": "all", "depth": depth / 1000}], doors=[])[1], [])

    def test_what_is_said_is_the_depth_or_the_sides_length(self):
        step = timber_step(colonnade=[{"sides": [0], "depth": 3.3}])
        (deep,) = [x for x in said_of(step, "slab_open") if "of the platform" in x[3]]
        self.assertIn("on side 0 it is 3.3 m from the line its tiles start on to the middle of the wall's line of "
                      "posts behind, no whole number of 1.5 m tiles", deep[3])
        self.assertEqual(deep[4], "make the colonnade's `depth` there 3 or 4.5 m: the platform is then whole tiles deep")
        _, open_ = self.platform(T.lay, ring=[(0, 0), (15800, 0), (15800, 9300), (0, 9300)],
                                 colonnade=[{"sides": [0], "depth": 3.0}], doors=[])
        (n,) = open_
        self.assertEqual(n["deep"], [])
        self.assertEqual([(d["side"], d["inside"], d["closes"]) for d in n["long"]],
                         [(0, 15500.0, [15300.0, 16800.0]), (2, 15500.0, [15300.0, 16800.0])])


class DoorOnTheAxis(unittest.TestCase):
    """court.py rule 4: what an odd number of bays left is said to be is true, and the counts
    offered are taken."""

    def test_a_door_of_two_bays_on_a_front_of_three(self):
        facts = {"door": 2, "posts": False}
        with self.assertRaises(KE) as c:
            K["on_axis"](3, facts, "hall", "the hall")
        self.assertIn("a front of 3 bays has a bay: the kit's door takes 2 bays, which leaves 1", c.exception.message)
        self.assertNotIn("joint", c.exception.message)
        self.assertEqual(c.exception.hint, "make it 2 or 4 bays across")
        for n in (2, 4):
            K["on_axis"](n, facts, "hall", "the hall")

    def test_a_door_wider_than_the_front(self):
        facts = {"door": 3, "posts": False}
        with self.assertRaises(KE) as c:
            K["on_axis"](1, facts, "hall", "the hall")
        self.assertIn("the hall is 1 bay across, and the kit's door takes 3 bays: the door is wider than the front",
                      c.exception.message)
        self.assertEqual(c.exception.hint, "make it 3 or 5 bays across")
        for n in (3, 5):
            K["on_axis"](n, facts, "hall", "the hall")
        with self.assertRaises(KE) as c:
            K["on_axis"](2, facts, "hall", "the hall")
        self.assertEqual(c.exception.hint, "make it 3 or 5 bays across")

    def test_an_even_front_of_posts_still_has_a_post_on_its_middle(self):
        with self.assertRaises(KE) as c:
            K["on_axis"](4, {"door": 1, "posts": True}, "hall", "the hall")
        self.assertIn("where a front of 4 bays has a post", c.exception.message)
        self.assertEqual(c.exception.hint, "make it 3 or 5 bays across")


class PostByHeight(unittest.TestCase):
    """Rule 6, "Pieces by storey": a post is chosen by its storey's height, as a panel is."""

    def rows(self):
        return [T.row("post_36", "post", (300, 300), 3600), T.row("post_42", "post", (300, 300), 4200),
                T.row("panel_36", "wall", (2700, 150), 3600, pitch=3.0),
                T.row("panel_42", "wall", (2700, 150), 4200, pitch=3.0)]

    def test_each_storey_gets_the_post_of_its_own_height(self):
        for high in (3600, 4200):
            kit = L["kit_pieces"](self.rows(), "ground", high)
            self.assertEqual((kit["wall"]["high"], kit["post"]["high"]), (high, high))

    def test_check_reads_the_post_of_the_storeys_height_as_lay_does(self):
        hall = {"name": "h", "kind": "building", "ring": T.HALL, "path": T.HALL,
                "attrs": {"kit": "timber", "storey_h": 4.2, "storeys": 1, "roof": "hip"}}
        self.assertEqual([r for r in O.judged(self.rows(), [hall]) if r[1] == "kit_height"], [])

    def test_check_says_a_post_that_is_not_the_storeys_height(self):
        rows = [r for r in self.rows() if r["id"] in ("post_42", "panel_36")]
        hall = {"name": "h", "kind": "building", "ring": T.HALL, "path": T.HALL,
                "attrs": {"kit": "timber", "storey_h": 3.6, "storeys": 1, "roof": "hip"}}
        said = [r for r in O.judged(rows, [hall]) if r[1] == "kit_height"]
        self.assertEqual([r[2] for r in said], ["post_42"])
        self.assertIn("`post_42` is 4.2 m high on the 3.6 m storeys of `h`", said[0][3])


class Fit(unittest.TestCase):
    """Rule 6: a stretch closes up to `FIT` longer than its pieces, and shorter only by what
    rounding its two corners to the millimetre takes off it."""

    def test_the_two_tolerances(self):
        self.assertEqual((L["FIT"], L["FIT_SHORT"]), (5, 1.5))
        for short in (0.4, 1, 1.5):
            self.assertEqual(L["fit"](6000 - short, 3000, 0), ((2, 0), None), short)
        for short in (2, 4, 5, 6):
            self.assertEqual(L["fit"](6000 - short, 3000, 0), (None, (3000, 6000)), short)
        for long in (1, 4, 5):
            self.assertEqual(L["fit"](6000 + long, 3000, 0), ((2, 0), None), long)
        self.assertEqual(L["fit"](6006, 3000, 0), (None, (6000, 9000)))

    def test_fillers_are_held_to_the_same(self):
        self.assertEqual(L["fit"](2999, 3000, 1500), ((1, 0), None))
        self.assertEqual(L["fit"](4499, 3000, 1500), ((1, 1), None))
        self.assertEqual(L["fit"](2996, 3000, 1500), (None, (1500, 3000)))
        self.assertEqual(L["fit"](4504, 3000, 1500), ((1, 1), None))

    def test_bays_between_posts_are_held_to_the_same(self):
        kit = O.colonnade_kit()
        self.assertEqual(L["post_bays"](14399, kit)[0], 5)
        self.assertEqual(L["post_bays"](14396, kit)[0], 0)
        self.assertEqual(L["post_bays"](14404, kit)[0], 5)
        self.assertEqual(L["post_bays"](14406, kit)[0], 0)

    def test_a_yard_side_a_few_millimetres_short_is_wall_open_again(self):
        """Before the review's mend a side 2 or 4 mm short closed, and the only word was the
        core's, of pieces in each other's place."""
        kits = lambda w, h: L["kit_pieces"](T.yard_kit(), w, h)                     # noqa: E731
        attrs = {"storeys": 0, "storey_h": 2.4, "kit": "yard"}
        for short, opens in ((0, 0), (1, 0), (2, 2), (4, 2)):
            ring = [(0, 0), (39600 - short, 0), (39600 - short, 36600), (0, 36600)]
            out = L["plan"]("y", ring, attrs, 10.0, None, None, kits=kits, foot=[])
            self.assertEqual(len(out["open"]), opens, short)

    def test_a_roofs_eave_is_held_to_the_same(self):
        self.assertTrue(L["closes"](6000, 2, 3000) and L["closes"](5999, 2, 3000) and L["closes"](6005, 2, 3000))
        self.assertFalse(L["closes"](5998, 2, 3000) or L["closes"](6006, 2, 3000))
        (n,) = T.notes(T.lay([(0, 0), (15297, 0), (15297, 9300), (0, 9300)]), "roof_none")
        self.assertIn("no whole number of 3 m slope pieces", n["why"])


class MirrorHeld(unittest.TestCase):
    """Rule 8: a mirrored `fill` makes two images only where the bays stand as images; where
    they do not the build says so (`mirror_none`)."""

    FILL = [{"sides": [2], "pattern": ["wall", "niche", "wall"], "mirror": True}]

    def order(self, ring, **attrs):
        a = {"colonnade": None, "doors": [], "roof": "flat", "fill": self.FILL, **attrs}
        return O.lay(ring=ring, **{k: v for k, v in a.items()})

    def words(self, out, side):
        got = []
        for r in sorted((r for r in out["runs"] if r["key"].startswith(f"t_s0_w{side}_")), key=lambda r: r["span"][0]):
            got += r.get("assets") or [r["asset"]] * r["count"]
        return got

    def test_a_side_with_a_half_bay_at_one_end_has_no_image_and_says_so(self):
        """The reviewer's case: 9.9 m is four bays and a half bay between its corners."""
        out = self.order([(0, 0), (9900, 0), (9900, 9000), (0, 9000)])
        self.assertEqual(self.words(out, 2), ["wall", "wall_niche", "wall", "wall_niche", "wall_half"])
        self.assertEqual([(n["side"], n["storey"]) for n in T.notes(out, "mirror_none")], [(2, 0)])

    def test_a_side_of_whole_bays_is_an_image_and_nothing_is_said(self):
        out = self.order([(0, 0), (9000, 0), (9000, 9000), (0, 9000)])
        self.assertEqual(self.words(out, 2), ["wall", "wall_niche", "wall_niche", "wall"])
        self.assertEqual(T.notes(out, "mirror_none"), [])

    def test_a_side_with_no_mirrored_fill_is_not_held(self):
        out = self.order([(0, 0), (9900, 0), (9900, 9000), (0, 9000)],
                         fill=[{"sides": [2], "pattern": ["wall", "niche", "wall"]}])
        self.assertEqual(T.notes(out, "mirror_none"), [])
        out = self.order([(0, 0), (9900, 0), (9900, 9000), (0, 9000)],
                         fill=[{"sides": [1], "pattern": ["wall", "niche"], "mirror": True}])
        self.assertEqual(T.notes(out, "mirror_none"), [])

    def test_a_door_off_the_middle_of_a_mirrored_side(self):
        fill = [{"sides": [0], "pattern": ["wall", "niche"], "mirror": True}]
        ring = [(0, 0), (12600, 0), (12600, 9000), (0, 9000)]           # six bays between the corners
        on = self.order(ring, fill=fill, doors=[{"storey": 0, "side": 0, "at": 6.3}])
        self.assertEqual(T.notes(on, "mirror_none"), [])
        off = self.order(ring, fill=fill, doors=[{"storey": 0, "side": 0, "at": 4.5}])
        self.assertEqual([n["side"] for n in T.notes(off, "mirror_none")], [0])

    def test_between_posts_a_door_off_the_middle_bay(self):
        fill = [{"sides": [0], "pattern": ["lattice", "wall"], "mirror": True}]
        on = T.lay(fill=fill, doors=[{"storey": 0, "side": 0, "at": 7.65}])
        self.assertEqual(T.notes(on, "mirror_none"), [])
        off = T.lay(fill=fill, doors=[{"storey": 0, "side": 0, "at": 4.65}])
        self.assertEqual([n["side"] for n in T.notes(off, "mirror_none")], [0])

    def test_it_is_said_once_a_side_as_a_warning(self):
        step = timber_step(fill=[{"sides": [0], "pattern": ["lattice", "wall"], "mirror": True}],
                           doors=[{"storey": 0, "side": 0, "at": 4.65}])
        (w,) = said_of(step, "mirror_none")
        self.assertEqual((w[0], w[5]), ("warning", {"side": 0, "storey": 0}))
        self.assertIn("do not stand as each other's image about its middle", w[3])


def bay_pieces(out, ring):
    """Every panel and post a building's wall runs lay, as `(asset, (x, y))` in mm: where the
    core stands a run's members along the storey's outline (rules.md 5.1, `ends = "half"`)."""
    lens = [T.math.dist(p, q) for p, q in zip(ring, ring[1:] + ring[:1])]
    total = sum(lens)

    def point(s):
        for (p, q), length in zip(zip(ring, ring[1:] + ring[:1]), lens):
            if s <= length:
                return (round(p[0] + (q[0] - p[0]) * s / length), round(p[1] + (q[1] - p[1]) * s / length))
            s -= length
        return ring[0]
    got = []
    for r in out["runs"]:
        if "_roof" in r["key"] or "_base" in r["key"]:
            continue
        a, b = r["span"][0] * total, r["span"][1] * total
        for k in range(r["count"]):
            got.append(((r.get("assets") or [r["asset"]] * r["count"])[k], point(a + (2 * k + 1) * (b - a) / (2 * r["count"]))))
    return sorted(got)


class WingPatterns(unittest.TestCase):
    """court.py rule 6 with a pattern of more than one word, which is what shows a pattern
    turned about from one reflected (examples/court's wings; before, every wing pattern was one
    word and no row or test could see `mirror = true`)."""

    WINGS = [{"bays": [3, 2], "from": 9, "off": 9,
              "fill": [{"sides": [0], "pattern": ["lattice"]}, {"sides": [1], "pattern": ["doors", "wall"]},
                       {"sides": [2], "pattern": ["lattice", "wall", "wall"]}]}]

    def twins(self, fills=None):
        b, _ = T.court(wings=self.WINGS)
        got = []
        for key in ("c_w0_l", "c_w0_r"):
            attrs = dict(b[key]["attrs"], porch=2, walk_out=3)
            if fills is not None:
                attrs["fill"] = fills(key, attrs["fill"])
            out = L["plan"](key, b[key]["ring"], attrs, 10.0, None, None,
                            kits=lambda w, h: L["kit_pieces"](T.timber(), w, h), foot=[])
            self.assertEqual(T.notes(out, "mirror_none") + out["open"], [])
            got.append(bay_pieces(out, b[key]["ring"]))
        return got

    def image(self, pieces):
        return sorted((a, (2 * T.GATE[0] - x, y)) for a, (x, y) in pieces)

    def test_the_twins_panels_are_images_bay_for_bay(self):
        left, right = self.twins()
        self.assertEqual(self.image(left), right)
        # and the pattern shows: the back wall reads lattice, wall, lattice from either end
        self.assertEqual(sorted(a for a, (x, y) in left if x == 16700 and a.startswith("panel")),
                         ["panel", "panel_lattice", "panel_lattice"])

    def test_read_from_one_end_the_twin_is_turned_about_not_reflected(self):
        """What `mirrored` is for: without it the same fills give twins that are not images."""
        plain = lambda key, fill: [{k: v for k, v in e.items() if k != "mirror"} for e in fill]      # noqa: E731
        left, right = self.twins(plain)
        self.assertNotEqual(self.image(left), right)


class ReadPlatform(unittest.TestCase):
    """check.py rule 1: the platform is read as a feature, and a room behind one is reached
    only through it."""

    def feats(self):
        out = O.lay()
        feats = [{"name": "lay#t_s0", "kind": "storey", "ring": O.CELLA, "path": O.CELLA, "attrs": {"level": 10.9}},
                 {"name": "lay#t_platform", "kind": "platform", "ring": O.TEMPLE, "path": O.TEMPLE,
                  "attrs": {"level": 10.9}}]
        for d in out["doorways"]:
            feats.append({"name": "lay#" + d["key"], "kind": "doorway", "ring": d["path"], "path": d["path"],
                          "attrs": {"sill_z": d["sill_z"]}})
        return feats

    def meet(self, a, b):
        return {"a": a, "b": b, "gap": 0, "connector_a": "foot", "connector_b": None, "x": 0, "y": 0, "turn": None}

    def test_a_platform_no_doorway_ends_on_leaves_the_room_unreached(self):
        inner = [self.meet("lay#t_d0_in.end", "lay#t_s0.floor")]
        said = [s for s in O.judged([], self.feats(), inner) if s[1] == "unreached"]
        self.assertEqual([s[2] for s in said], ["lay#t_s0"])
        # with the platform not among the features, the same walk would start on the terrain
        bare = [f for f in self.feats() if f["kind"] != "platform"]
        self.assertEqual([s for s in O.judged([], bare, inner) if s[1] == "unreached"], [])

    def test_the_step_reads_the_platform_among_its_features(self):
        asked = []

        class Step:
            def features(self, select, columns=()):
                asked.append((select, list(columns)))
                fl = O._FL({"name": ["p"] if select == "kind:platform" else [], "geometry": [2], "coords": O.TEMPLE})
                fl["level"], fl["level?"] = [10.9], [1]
                return fl
        got = O.C["read_features"](Step())
        self.assertIn(("kind:platform", ["level"]), asked)
        self.assertEqual([(f["name"], f["kind"], f["attrs"]) for f in got], [("p", "platform", {"level": 10.9})])


class Survivors(unittest.TestCase):
    """The single breakages of K8's code that the review's mutation run found no test for, each
    with the case that now fails under it."""

    ROWS = O.PostWalls.ROWS
    HALL = O.PostWalls.HALL

    def walls(self, ring=None, footprint=None, doors=(), rows=None, crossers=(), notes=None):
        ring = ring or self.HALL
        kit = L["kit_pieces"](self.ROWS if rows is None else rows)
        return L["wall_plan"](ring, footprint or ring, list(doors), kit, "h_s0", "h_s0", crossers, 100,
                              [] if notes is None else notes, None, "h")

    def laid(self, runs, start):
        return {r["key"]: (r["count"], O.stations(r, self.HALL)) for r in runs if r["key"].startswith(start)}

    # --- lay.py rule 15

    def test_a_narrow_path_through_a_post_takes_the_post_and_no_panel(self):
        """What is taken is what the path's gap overlaps: a panel between its two posts' faces,
        a post across its own width. A gap inside one post's width takes that post alone."""
        path = [(6150, -2000), (6150, 2000)]
        runs, _, _ = self.walls(crossers=[(path, 50, ("flight", "f0"))])
        self.assertEqual(self.laid(runs, "h_s0_w0"), {"h_s0_w0_0": (5, [1650, 4650, 7650, 10650, 13650])})
        self.assertEqual(self.laid(runs, "h_s0_p0"), {"h_s0_p0": (1, [3150]), "h_s0_p0_1": (2, [9150, 12150])})

    def test_a_door_is_held_by_the_stretch_it_stands_in_only(self):
        """A side cut in two by a stairwell is two stretches: the door in the second is not the
        first's, which keeps its panel."""
        ring = L["notch_ring"](self.HALL, (0, 3300, 9000, 1500))
        door = L["door_plan"](self.HALL, {"storey": 0, "side": 0, "at": 10.65, "width": 1.2}, 2000, 0)
        runs, insts, opens = self.walls(ring=ring, footprint=self.HALL, doors=[door])
        self.assertEqual(opens, [])
        on0 = sorted((r["key"], r["count"]) for r in runs if r["key"].startswith("h_s0_w0_"))
        self.assertEqual([c for _, c in on0], [1])                        # the first stretch's one bay
        self.assertEqual([i["at"] for i in insts if i["key"] == "h_s0_door0"], [(10650, 150)])

    def test_a_door_that_runs_into_the_post_after_it_is_refused(self):
        rows = [r for r in self.ROWS if r["id"] != "frame"]              # no door piece: only the posts hold it
        for at, ok in ((4.65, True), (5.2, True), (5.5, False), (3.8, False)):
            door = L["door_plan"](self.HALL, {"storey": 0, "side": 0, "at": at, "width": 1.2}, 2000, 0)
            if ok:
                self.walls(doors=[door], rows=rows)
                continue
            with self.assertRaises(PE) as e:
                self.walls(doors=[door], rows=rows)
            self.assertIn("where a post stands in its opening (1.2 m)", e.exception.message)

    def test_a_side_shorter_than_its_two_corner_posts(self):
        notes = []
        ring = [(0, 0), (500, 0), (500, 9300), (0, 9300)]
        runs, _, opens = self.walls(ring=ring, notes=notes)
        self.assertEqual([(n["code"], n["side"], n["length"], n["takes"]) for n in notes],
                         [("side_short", 0, 500.0, 600.0), ("side_short", 2, 500.0, 600.0)])
        self.assertEqual([r["key"] for r in runs if "_w0" in r["key"] or "_w2" in r["key"]], [])

    def corner_kit(self):
        return self.ROWS + [O.row("knee", "corner", (600, 600), 3000, shared={}, reach=0.6)]

    def test_a_corner_piece_with_its_own_reach_moves_the_first_post(self):
        """A kit whose corner is not its post: the corner takes its own reach of each wall, and
        the first joint is half a post before the corner's inner face."""
        kit = L["kit_pieces"](self.corner_kit())
        n, joints, _ = L["post_bays"](15900, kit)                          # five bays, a post, and two corners of 0.6
        self.assertEqual((n, joints[0], joints[-1]), (5, 450.0, 15450.0))
        self.assertEqual(L["post_bays"](15300, kit)[0], 0)                 # the post kit's own length no longer closes

    def test_its_reach_is_what_a_stretch_that_does_not_close_is_measured_by(self):
        ring = [(0, 0), (16400, 0), (16400, 9900), (0, 9900)]
        _, insts, opens = self.walls(ring=ring, rows=self.corner_kit())
        (side0,) = [o for o in opens if o["side"] == 0]
        self.assertEqual((side0["length"], side0["near"]), (15200.0, (14700.0, 17700.0)))
        # and the corner piece stands half its own reach in from the vertex, both ways
        self.assertEqual([i["at"] for i in insts if i["key"] == "h_s0_c0"], [(300, 300)])

    # --- lay.py rule 16

    def test_a_door_past_the_far_end_of_the_wall_behind(self):
        with self.assertRaises(PE) as e:
            T.lay(colonnade=[{"sides": [0, 1], "depth": 3.0}], doors=[{"storey": 0, "side": 0, "at": 13.65}])
        self.assertIn("door 0 is 13.65 m along side 0, and the wall behind the colonnade stands from 0 to 12.3 m",
                      e.exception.message)

    # --- lay.py rule 17

    def test_over_an_l_the_roofs_depth_is_the_length_two_sides_share_with_a_slope(self):
        ell = [(0, 0), (18000, 0), (18000, 9000), (9000, 9000), (9000, 12000), (0, 12000)]     # its shortest side is 3 m
        kit, why = L["roof_for"](L["kit_pieces"](T.timber()), ell)
        self.assertIsNone(why)
        self.assertEqual(kit["roof_slope"]["id"], "roof_slope_45")

    def test_a_roof_with_no_eaves_does_not_say_it_stands_beyond_the_outline(self):
        plain = {k: v for k, v in T.SHARED.items() if k != "eaves"}
        (n,) = T.notes(T.lay([(0, 0), (15300, 0), (15300, 7500), (0, 7500)], rows=T.timber(shared=plain)), "roof_none")
        self.assertIn("the kit's slope pieces are made for roofs 9 and 12 m deep", n["why"])
        self.assertNotIn("beyond the outline", n["why"])
        (n,) = T.notes(T.lay([(0, 0), (15300, 0), (15300, 7500), (0, 7500)]), "roof_none")
        self.assertIn("the roof stands 1.35 m beyond the outline on every side", n["why"])

    # --- lay.py rule 18

    def yard(self, rows=None, **attrs):
        rows = T.yard_kit() if rows is None else rows
        a = {"storeys": 0, "storey_h": 2.4, "kit": "yard", **attrs}
        return L["plan"]("y", T.Yard.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h), foot=[])

    def test_a_yards_pieces_are_chosen_by_its_height(self):
        rows = T.yard_kit() + [T.row("yard_wall_tall", "wall", (3000, 300), 3600, shared=T.YARD, taxon="yard", pitch=3.0)]
        self.assertEqual({r["asset"] for r in self.yard(rows)["runs"]}, {"yard_wall"})
        self.assertEqual({r["asset"] for r in self.yard(rows, storey_h=3.6)["runs"]}, {"yard_wall_tall"})

    def test_a_bay_named_where_a_yard_has_none_is_said(self):
        out = self.yard(bays=[{"storey": 0, "side": 0, "at": 0.1, "variant": "wall"}])
        self.assertEqual([(n["code"], n["side"], n["at_m"]) for n in out["notes"]], [("bay_none", 0, 0.1)])
        self.assertEqual(self.yard(bays=[{"storey": 0, "side": 0, "at": 1.8, "variant": "wall"}])["notes"], [])

    # --- court.py

    def test_a_kit_of_whole_wall_pieces_is_sized_by_its_stated_wall_and_its_first_storey(self):
        sh = {"wall": 0.4, "storey": [2.4, 3.0]}
        rows = [T.row("w", "wall", (3000, 300), 2700, shared=sh, taxon="k", pitch=3.0)]
        f = K["kit_facts"](rows, "k")
        self.assertEqual((f["wall"], f["ends"], f["storey"]), (400, 800, 2400))
        f = K["kit_facts"]([dict(r, shared={}) for r in rows], "k")
        self.assertEqual((f["wall"], f["ends"], f["storey"]), (300, 600, 2700))

    def test_a_hall_past_the_yards_far_wall(self):
        _, notes = T.court(hall={"bays": [5, 3], "from": 28})
        self.assertEqual([(n["key"], n["wall"], n["eaves"], n["by"], n["dropped"]) for n in notes],
                         [("c_hall", True, False, 1000.0, True)])

    def test_a_wings_eaves_over_the_yards_front_wall(self):
        _, notes = T.court(wings=[{"bays": [3, 2], "from": 1, "off": 9}])
        self.assertEqual({(n["key"], n["wall"], n["eaves"], n["by"], n["dropped"]) for n in notes},
                         {("c_w0_l", True, True, 650.0, False), ("c_w0_r", True, True, 650.0, False)})

    def test_a_note_outside_the_worlds_extent_has_no_position(self):
        def run(extent):
            s = T.FakeStep({"c": ([T.GATE], {**T.COURT, "wings": [{"bays": [3, 2], "from": 13, "off": 9}]})})
            s.id, s.grid, kws = "courts", {"extent_mm": extent}, []
            s.warning = lambda code, subject, message, hint, **kw: kws.append(kw)
            K["generate"](s)
            return ["position" in kw for kw in kws]
        self.assertEqual(run((0, 0, 64000, 64000)), [True, True])
        self.assertEqual(run((0, 0, 20000, 64000)), [True, False])          # the right wing's middle is 44.15 m east

    # --- check.py

    def judged(self, rows, code):
        return [r for r in O.judged(rows) if r[1] == code]

    def test_hips_with_no_slope_are_said(self):
        rows = T.timber(drop=("roof_slope_6", "roof_slope_45"))
        said = self.judged(rows, "kit_deck")
        self.assertEqual(len(said), 1)
        self.assertIn("has `roof_hip_45` and no `role:roof_slope` piece", said[0][3])

    def test_a_colonnades_post_is_held_to_the_storey_by_its_top_too(self):
        short = T.row("veranda_post", ["post", "corner"], (300, 300), 3000, ("storey:colonnade",))
        said = [r for r in self.judged(T.timber(swap={"veranda_post": short}), "kit_stated") if r[2] == "veranda_post"]
        self.assertEqual(len(said), 1)
        self.assertIn("stands 3 m high where the kit states a storey of 3.6 m", said[0][3])

    def test_a_floor_tiles_measured_plan_is_held_to_the_unit(self):
        tile = T.row("tile", "floor", (1420, 1420), 100, pitch=1.5)            # 0.07 m off the unit, each way
        said = [r for r in self.judged(T.timber(swap={"tile": tile}), "kit_unit") if r[2] == "tile"]
        self.assertEqual(len(said), 1)
        self.assertIn("measured length of 1.42 m and a measured thickness of 1.42 m", said[0][3])

    def test_a_slope_made_for_a_building_of_no_bays_is_said(self):
        """Span 1.5 m under eaves of 1.35 m roofs 0.3 m: one post and no bay."""
        rows = T.timber() + [T.row("roof_slope_15", "roof_slope", (3000, 1500), 600, pitch=3.0, span=1.5),
                             T.row("roof_hip_15", "roof_hip", (1500, 1500), 600, span=1.5)]
        said = [r for r in self.judged(rows, "kit_stated") if r[2] == "roof_slope_15"]
        self.assertEqual(len(said), 1)
        self.assertIn("roofs a building 0.3 m deep", said[0][3])

    def test_under_eaves_a_kit_with_a_half_bay_is_sized_by_it(self):
        """Walls 0.3 m thick, a 3 m bay and a 1.5 m half bay: a building may be 5.1 m deep, which
        a slope of span 3.15 m under eaves of 0.6 m roofs."""
        sh = {"wall": 0.3, "storey": 3.0, "eaves": 0.6}

        def kit(filler):
            rows = [T.row("wall", "wall", (3000, 300), 3000, shared=sh, pitch=3.0),
                    T.row("corner", "corner", (300, 300), 3000, shared=sh),
                    T.row("slope", "roof_slope", (3000, 3150), 1500, shared=sh, pitch=3.0, span=3.15),
                    T.row("hip", "roof_hip", (3150, 3150), 1500, shared=sh, span=3.15)]
            return rows + ([T.row("half", "filler", (1500, 300), 3000, shared=sh, pitch=1.5)] if filler else [])
        self.assertEqual(self.judged(kit(True), "kit_stated"), [])
        said = self.judged(kit(False), "kit_stated")
        self.assertEqual([r[2] for r in said], ["slope"])
        self.assertIn("whole pieces of 3 m and two corners of 0.3 m", said[0][3])


if __name__ == "__main__":
    unittest.main()
