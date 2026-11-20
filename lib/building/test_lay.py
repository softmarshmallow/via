"""Tests for lay.py: room layout, doors, stairs, walls, and the step through a stand-in SDK.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The house is surfaces.md 6.4's: the plan must give exactly its storeys, stair and doorway.
"""

import json
import math
import os
import runpy
import shutil
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
L = runpy.run_path(os.path.join(HERE, "lay.py"))
ParamError = L["ParamError"]
GRID = {"extent_mm": [0, 0, 64000, 64000], "resolution_mm": 1000, "nx": 65, "ny": 65}
HALL = [(20000, 20000), (32000, 20000), (32000, 28000), (20000, 28000)]
ATTRS = {"storeys": 2, "storey_h": 3.0, "plinth": 0.3, "porch": 4, "walk_out": 12,
         "stair": {"t": 0.025, "width": 1, "run": 5}, "doors": [{"storey": 0, "t": 0.4, "width": 1}]}
# A kit whose pieces close every stretch of the hall: 1 m bays, 0.1 m fillers, 0.3 m thick.
KIT = {"wall": {"id": "bay", "pitch": 1000, "thick": 300}, "filler": {"id": "strip", "pitch": 100, "thick": 300},
       "corner": {"id": "corner", "pitch": 300, "thick": 300}, "door": {"id": "leaf", "pitch": 1000, "thick": 300}}


def plan(attrs=None, ring=HALL, ground=10.0, kit=None, grid=GRID):
    a = dict(ATTRS)
    a.update(attrs or {})
    return L["plan"]("hall", ring, a, ground, grid, kit)


def same_ring(a, b):
    """Equal up to where the ring starts."""
    a, b = list(a), list(b)
    if len(a) != len(b):
        return False
    return any(a[i:] + a[:i] == b for i in range(len(a)))


def area(ring):
    return abs(L["area2"](ring)) / 2


class Stairs(unittest.TestCase):
    def test_the_worked_example(self):
        """surfaces.md 6.4: hall_g, hall_1 (the south-west notch), the stair, the levels."""
        out = plan()
        s0, s1 = out["storeys"]
        self.assertTrue(same_ring(s0["region"], HALL))
        self.assertTrue(same_ring(s1["region"], [(26000, 20000), (32000, 20000), (32000, 28000), (20000, 28000),
                                                 (20000, 21200), (26000, 21200)]))
        self.assertEqual((s0["level"], s1["level"]), (10.3, 13.3))
        (f,) = out["flights"]
        self.assertEqual(f["path"], [(21000, 20600), (26000, 20600)])
        self.assertEqual((f["starts_at"], f["ends_at"]), ("hall_s0.floor", "hall_s1.floor"))
        self.assertEqual((f["z0"], f["z1"]), (10.3, 13.3))

    def test_each_flight_starts_on_its_floor_and_ends_on_the_next(self):
        """Three and four storeys switch back: every start inside the lower storey's region and
        every end inside the upper's (edges count, surfaces.md 1.2), never the start of the next
        flight inside a notch."""
        for n in (3, 4):
            out = plan({"storeys": n})
            rings = {s["key"]: s["region"] for s in out["storeys"]}
            self.assertEqual(len(out["flights"]), n - 1)
            for f in out["flights"]:
                lo, hi = f["starts_at"][:-6], f["ends_at"][:-6]
                self.assertTrue(L["inside"](rings[lo], f["path"][0]), (n, f))
                self.assertTrue(L["inside"](rings[hi], f["path"][-1]), (n, f))
            f0, f1 = out["flights"][:2]
            self.assertEqual(f1["path"], [(26000, 21700), (21000, 21700)])   # lane 1, back
            self.assertEqual(f0["path"][0][1], 20600)

    def test_a_turned_building_keeps_every_end_on_its_floor(self):
        """Turned off the lattice's axes (every 5 degrees to 90, corners rounded to the mm): every
        flight and doorway end inside its floor; without `onto`, 18 ends over these turns rounded to
        just outside (M5 close, a fresh agent's house turned 20 degrees: `surface.outside`)."""
        import math
        for deg in range(0, 91, 5):
            a = math.radians(deg)
            ring = [(round(26000 + (x - 26000) * math.cos(a) - (y - 24000) * math.sin(a)),
                     round(24000 + (x - 26000) * math.sin(a) + (y - 24000) * math.cos(a))) for x, y in HALL]
            out = plan({"storeys": 3, "stair": {"t": 0.06, "width": 1, "run": 5}}, ring=ring)
            rings = {s["key"]: s["region"] for s in out["storeys"]}
            for f in out["flights"]:
                self.assertTrue(L["inside"](rings[f["starts_at"][:-6]], f["path"][0]), (deg, f))
                self.assertTrue(L["inside"](rings[f["ends_at"][:-6]], f["path"][-1]), (deg, f))
            for d in out["doorways"]:
                self.assertTrue(L["inside"](rings[d["ends_at"][:-6]], d["path"][-1]), (deg, d))

    def test_notch_of_an_odd_flight_is_two_lanes_deep(self):
        out = plan({"storeys": 3})
        s2 = out["storeys"][2]["region"]
        self.assertIn((21000, 22300), s2)       # 2 * 1 m + 3 * 0.1 m in from the wall
        self.assertIn((27000, 22300), s2)       # s1 + the landing
        self.assertFalse(L["inside"](s2, (23000, 21000)))

    def test_levels_are_whole_millimetres(self):
        out = plan({"storey_h": 3.2}, ground=10.12345)
        self.assertEqual([s["level"] for s in out["storeys"]], [10.423, 13.623])

    def test_a_stair_is_needed_and_must_stay_on_its_edge(self):
        with self.assertRaises(ParamError):
            plan({"stair": None})
        with self.assertRaises(ParamError):
            plan({"stair": {"t": 0.2, "width": 1, "run": 5}})   # 8 m along a 12 m edge, past it
        with self.assertRaises(ParamError):
            plan({"stair": {"t": 0.32, "width": 1, "run": 5}})  # the 8 m east edge: past it

    def test_a_stairwell_stub_before_a_corner_is_refused_by_t(self):
        """The landing 1 m before the foot: a foot 1.08 m from the corner leaves 0.08 m of wall,
        which no piece closes (M7 close); 1.0 m reaches the corner and 1.3 m leaves 0.3 m."""
        with self.assertRaises(ParamError) as e:
            plan({"stair": {"t": 0.027, "width": 1, "run": 5}}, kit=KIT)
        self.assertIn("0.080 m from the corner", str(e.exception.args))
        for t in (0.025, 0.0325):
            plan({"stair": {"t": t, "width": 1, "run": 5}}, kit=KIT)


class Doors(unittest.TestCase):
    def test_the_worked_example(self):
        out = plan()
        (d,) = out["doorways"]
        self.assertEqual(d["path"], [(36000, 24000), (32000, 24000)])
        self.assertEqual((d["ends_at"], d["sill_z"]), ("hall_s0.floor", 10.3))
        (p,) = out["footpaths"]
        self.assertEqual((p["path"], p["ends_at"]), ([(48000, 24000)], "hall_d0.start"))

    def test_a_doorway_is_square_to_a_slanted_wall(self):
        ring = [(0, 0), (10000, 0), (16000, 8000), (6000, 8000)]
        out = plan({"storeys": 1, "doors": [{"t": 0.3, "width": 1}], "porch": 2}, ring=ring)
        a, b = out["doorways"][0]["path"]
        wall = (6000, 8000)
        self.assertAlmostEqual((a[0] - b[0]) * wall[0] + (a[1] - b[1]) * wall[1], 0, delta=10000 * 1.5)
        self.assertAlmostEqual(math.dist(a, b), 2000, delta=1.5)

    def test_flat_covers_the_posts_round_the_end(self):
        coarse = dict(GRID, resolution_mm=2500)
        out = plan(grid=coarse)
        self.assertGreaterEqual(out["doorways"][0]["flat"], 2.5 * math.sqrt(2))

    def test_refused_doors(self):
        for door in ({"storey": 1, "t": 0.4}, {"t": 0.0, "width": 1}, {"t": 0.05, "width": 1}):
            with self.assertRaises(ParamError, msg=door):
                plan({"doors": [door]})


class Rooms(unittest.TestCase):
    def test_cut_across_the_long_side_at_the_shares(self):
        out = plan({"rooms": [{"storey": 0, "shares": [2, 1]}]})
        r0, r1 = out["rooms"]
        self.assertTrue(same_ring(r0["region"], [(20000, 20000), (28000, 20000), (28000, 28000), (20000, 28000)]))
        self.assertTrue(same_ring(r1["region"], [(28000, 20000), (32000, 20000), (32000, 28000), (28000, 28000)]))
        self.assertEqual(r0["storey"], "hall_s0")
        (o,) = out["openings"]
        self.assertEqual((o["at"], o["rooms"]), ((28000, 24000), (0, 1)))

    def test_rooms_of_a_notched_storey_fill_it_with_no_spike(self):
        out = plan({"rooms": [{"storey": 1, "shares": [1, 1]}]})
        storey = out["storeys"][1]["region"]
        rooms = [r["region"] for r in out["rooms"]]
        self.assertAlmostEqual(sum(area(r) for r in rooms), area(storey))
        for r in rooms:
            for i in range(len(r)):
                a, p, b = r[i - 1], r[i], r[(i + 1) % len(r)]
                self.assertNotEqual((p[0] - a[0]) * (b[1] - p[1]) - (p[1] - a[1]) * (b[0] - p[0]), 0, r)

    def test_a_room_in_two_pieces_is_refused(self):
        """A footprint like a C opening east: a cut across its long side through both arms."""
        c = [(0, 0), (12000, 0), (12000, 3000), (4000, 3000), (4000, 5000), (12000, 5000), (12000, 8000),
             (0, 8000)]
        with self.assertRaises(ParamError):
            L["room_plan"](c, c, [1, 1])          # x = 6 m crosses the outline 4 times
        rooms, openings = L["room_plan"](c, c, [1, 10])   # x = 1.09 m, in the back of the C
        self.assertEqual(len(rooms), 2)
        self.assertEqual(len(openings), 1)

    def test_bad_shares(self):
        for shares in ([], [0, 1], [-1, 2]):
            with self.assertRaises(ParamError):
                plan({"rooms": [{"storey": 0, "shares": shares}]})


class Walls(unittest.TestCase):
    def test_fit(self):
        self.assertEqual(L["fit"](11400, 1000, 100)[0], (11, 4))
        self.assertEqual(L["fit"](6000, 6000, 0)[0], (1, 0))
        got, near = L["fit"](6440, 6000, 0)
        self.assertIsNone(got)
        self.assertEqual(near, (6000, 12000))

    def test_runs_close_the_ground_storey_leaving_the_door(self):
        out = plan(kit=KIT)
        self.assertEqual(out["open"], [])
        runs = [r for r in out["runs"] if r["ref"] == "hall_s0"]
        covered = sum((r["span"][1] - r["span"][0]) * 40000 for r in runs)
        # less 4 corners and the door's gap: 1 m wide plus the 0.1 m margin is 1.1 m, which leaves
        # 3.15 m each side, not a whole number of 0.1 m fillers; each side gives 0.05 m to 3.1 m.
        self.assertAlmostEqual(covered, 40000 - 8 * 300 - 1200, delta=1)
        leaf = [i for i in out["instances"] if i["asset"] == "leaf"]
        self.assertEqual([(i["at"], i["face"], i["surface"]) for i in leaf], [((31850, 24000), 90.0, "hall_s0.floor")])
        corners = [i for i in out["instances"] if i["asset"] == "corner" and i["surface"] == "hall_s0.floor"]
        self.assertEqual(sorted(i["at"] for i in corners),
                         [(20150, 20150), (20150, 27850), (31850, 20150), (31850, 27850)])

    def test_a_stretch_that_does_not_close_is_open(self):
        kit = dict(KIT, filler=None)
        out = plan(kit={k: v for k, v in kit.items() if v})
        self.assertTrue(out["open"])
        self.assertTrue(all(o["near"][0] < o["length"] < o["near"][1] for o in out["open"]))

    def clear_at(self, out, s, asset_half=None):
        """The clear width (mm) round arc length `s` of the ground storey's outline, between the
        ends of its runs' pieces: a run of `count` members over its span, each `asset_half` mm
        each side of its member point (half its pitch by default)."""
        lo, hi = -math.inf, math.inf
        for r in (r for r in out["runs"] if r["ref"] == "hall_s0"):
            a, b = r["span"][0] * 40000, r["span"][1] * 40000
            pitch = (b - a) / r["count"]
            half = asset_half.get(r["asset"], pitch / 2) if asset_half else pitch / 2
            for k in range(r["count"]):
                m = a + (2 * k + 1) * (b - a) / (2 * r["count"])
                if m + half <= s:
                    lo = max(lo, m + half)
                elif m - half >= s:
                    hi = min(hi, m - half)
                else:
                    return 0
        return hi - lo

    def test_the_door_is_left_its_width_and_the_margin(self):
        """Rule 6: between the pieces each side of the door (t = 0.4, 16 m along) there is at
        least the door's 1 m and the 0.1 m margin; with a margin of 0.5 m, 1.5 m."""
        self.assertGreaterEqual(self.clear_at(plan(kit=KIT), 16000), 1100)
        self.assertGreaterEqual(self.clear_at(L["plan"]("hall", HALL, ATTRS, 10.0, GRID, KIT, margin=500), 16000),
                                1500)

    def test_a_piece_wider_than_its_pitch_is_pulled_back(self):
        """A bay 1.2 m wide at a 1 m pitch reaches 0.1 m past each end of its run: the gap is
        widened by that, so the clear width between the pieces' bounds still holds 1.1 m. Without
        the pull back it would be 0.9 m (the station's front door came out 0.20 m)."""
        kit = dict(KIT, wall=dict(KIT["wall"], width=1200))
        out = plan(kit=kit)
        self.assertEqual(L["overhang"](kit), 100)
        self.assertGreaterEqual(self.clear_at(out, 16000, {"bay": 600, "strip": 50}), 1100)

    def test_a_flight_stands_clear_of_the_wall(self):
        """Rule 3: with a 0.3 m wall the flight's lane is measured from the wall's inner face, so
        its strip (centre 0.9 m in, 1 m wide) starts 0.1 m clear of the wall; and the storey above
        loses the 0.3 m too."""
        out = plan(kit=KIT)
        (f,) = out["flights"]
        self.assertEqual(f["path"], [(21000, 20900), (26000, 20900)])
        self.assertEqual(f["path"][0][1] - 500 - 20000, 300 + 100)
        self.assertTrue(same_ring(out["storeys"][1]["region"],
                                  [(26000, 20000), (32000, 20000), (32000, 28000), (20000, 28000),
                                   (20000, 21500), (26000, 21500)]))

    def test_a_flight_crossing_the_outline_gets_a_gap(self):
        """A flight (1 m) that crosses a wall stretch, square or along its band, is left its width
        and the margin; one that stays a lane in from the wall is left none."""
        e = L["edges"]([(0, 0), (10000, 0), (10000, 8000), (0, 8000)])[0]
        self.assertEqual(L["crossing_gap"](e, [(4000, -2000), (4000, 3000)], 1000, 300, 550), (3450, 4550))
        self.assertEqual(L["crossing_gap"](e, [(2000, 600), (6000, 600)], 1000, 300, 550), (1450, 6550))
        self.assertIsNone(L["crossing_gap"](e, [(2000, 900), (6000, 900)], 1000, 300, 550))
        ring = [(0, 0), (10000, 0), (10000, 8000), (0, 8000)]
        kit = {k: KIT[k] for k in ("wall", "filler")}
        runs, _, opens = L["wall_plan"](ring, ring, [], kit, "b", "b_s0", [([(4000, -2000), (4000, 3000)], 1000)])
        bottom = sorted((r["span"][0] * 36000, r["span"][1] * 36000) for r in runs if "_w0_" in r["key"])
        self.assertEqual(opens, [])
        self.assertLessEqual(bottom[0][1], 3450 + 1)
        self.assertGreaterEqual(bottom[-1][0], 4550 - 1)

    def test_a_corner_takes_its_reach_from_each_wall(self):
        """Rule 6, corners: an outside corner that says `reach` shortens both its walls by it and
        its box is `reach` square in the corner; without `reach` it is a post as thick as the wall."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        post = {"wall": {"id": "bay", "pitch": 3000, "thick": 300}, "corner": {"id": "post", "pitch": 300, "thick": 300}}
        runs, insts, opens = L["wall_plan"](ring, ring, [], post, "b", "b_s0")
        self.assertEqual(opens, [])
        self.assertEqual(sorted(r["count"] for r in runs), [3, 3, 4, 4])
        self.assertEqual(sorted(i["at"] for i in insts), [(150, 150), (150, 9450), (12450, 150), (12450, 9450)])
        legged = dict(post, corner={"id": "ell", "pitch": 1800, "thick": 1800, "reach": 1800})
        ring = [(0, 0), (15600, 0), (15600, 9600), (0, 9600)]          # 15.6 - 3.6 = 12 m, 9.6 - 3.6 = 6 m
        runs, insts, opens = L["wall_plan"](ring, ring, [], legged, "b", "b_s0")
        self.assertEqual(opens, [])
        self.assertEqual(sorted(r["count"] for r in runs), [2, 2, 4, 4])
        self.assertEqual(sorted(i["at"] for i in insts), [(900, 900), (900, 8700), (14700, 900), (14700, 8700)])

    ELL = [(0, 0), (9600, 0), (9600, 5100), (5100, 5100), (5100, 9600), (0, 9600)]   # inside corner at (5100, 5100)
    HALF = {"wall": {"id": "bay", "pitch": 3000, "thick": 300}, "filler": {"id": "half", "pitch": 1500, "thick": 300},
            "corner": {"id": "post", "pitch": 300, "thick": 300}}

    def test_an_ell_does_not_close_on_posts(self):
        """With a post at every corner, the two sides at the inside corner hold their length less
        one post, the others less two: 4.5 m sides hold 4.2 m and stay open, whatever the L."""
        runs, insts, opens = L["wall_plan"](self.ELL, self.ELL, [], self.HALF, "b", "b_s0")
        self.assertEqual(sorted(o["length"] for o in opens), [4200, 4200])
        # the post still stands in the notch the two walls leave, as before
        self.assertIn((4950, 4950), [i["at"] for i in insts])

    def test_an_ell_closes_with_an_inside_corner(self):
        """`role:corner_in` reaching 1.2 m: with the post's 0.3 m outside that is one half bay, so
        every side of the L is whole pieces. It is placed by the middle of the notch, as the
        post is: a point on the floor, where the middle of its own box is not."""
        kit = dict(self.HALF, corner_in={"id": "inner", "pitch": 1500, "thick": 1500, "reach": 1200})
        runs, insts, opens = L["wall_plan"](self.ELL, self.ELL, [], kit, "b", "b_s0")
        self.assertEqual(opens, [])
        inner = [i for i in insts if i["asset"] == "inner"]
        self.assertEqual([(i["at"], i["face"]) for i in inner], [((4950, 4950), 90.0)])
        self.assertEqual(len([i for i in insts if i["asset"] == "post"]), 5)
        total = sum(L["edges"](self.ELL)[k]["L"] for k in range(6))
        laid = sum((r["span"][1] - r["span"][0]) * total for r in runs)
        # the outline less five posts on two walls each and the inside corner's two reaches
        self.assertAlmostEqual(laid, total - 5 * 2 * 300 - 2 * 1200, delta=1)

    def test_fillers_only_is_noted(self):
        """A bay whose pitch fits no stretch: every wall closes in half bays, and each stretch a
        bay or more long is noted."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        kit = dict(self.HALF, wall={"id": "bay", "pitch": 3200, "thick": 300})
        notes = []
        runs, _, opens = L["wall_plan"](ring, ring, [], kit, "b", "b_s0", notes=notes)
        self.assertEqual(opens, [])
        self.assertEqual({r["asset"] for r in runs}, {"half"})
        self.assertEqual(sorted((n["code"], n["fillers"]) for n in notes),
                         [("fillers_only", 6), ("fillers_only", 6), ("fillers_only", 8), ("fillers_only", 8)])
        notes = []
        L["wall_plan"](ring, ring, [], self.HALF, "b", "b_s0", notes=notes)
        self.assertEqual(notes, [])

    def test_a_gap_wider_than_its_door_is_noted(self):
        """A door 1.6 m wide in a kit whose door piece is 1.5 m: the walls either side are cut back
        to lengths that close and leave 4.5 m, 2.8 m more than the 1.7 m the door needs. The same
        door at 1.4 m is left 1.5 m, its piece's width, and nothing is noted."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        kit = dict(self.HALF, door={"id": "frame", "pitch": 1500, "thick": 300, "width": 1500})
        path = [(4050, -2000), (4050, 0)]
        for width, want in ((1400, []), (1600, [("gap_wide", 4500, 1700)])):
            notes = []
            L["wall_plan"](ring, ring, [], kit, "b", "b_s0", [(path, width, ("door", "b_d0"))], notes=notes)
            self.assertEqual([(n["code"], round(n["left"]), round(n["need"])) for n in notes], want)

    def test_a_place_by_side_and_distance(self):
        """`side` and `at` name the same place as the `t` that works out to it, and keep it when
        another side grows; `t` does not."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        es = L["edges"](ring)
        self.assertEqual(L["where"](es, {"side": 0, "at": 4.05}, "doors"), (0, 4050.0))
        k, s = L["where"](es, {"t": 4.05 / 44.4}, "doors")
        self.assertEqual(k, 0)
        self.assertAlmostEqual(s, 4050, delta=0.01)
        self.assertEqual(L["where"](es, {"side": 2, "at": 2.8}, "stair"), (2, 2800.0))
        longer = L["edges"]([(0, 0), (13600, 0), (13600, 9600), (0, 9600)])
        self.assertEqual(L["where"](longer, {"side": 0, "at": 4.05}, "doors"), (0, 4050.0))
        self.assertAlmostEqual(L["where"](longer, {"t": 4.05 / 44.4}, "doors")[1], 4232.4, delta=0.1)
        for bad in ({"side": 4, "at": 1}, {"side": 0, "at": 13}, {"side": 0}, {"side": 0, "at": 1, "t": 0.1}, {},
                    {"side": 1.7, "at": 1}, {"side": -1, "at": 1}, {"side": True, "at": 1}, {"side": 0, "at": True},
                    {"side": "0", "at": 1}, {"side": 0, "at": [1]}, {"side": 0, "at": float("nan")},
                    {"t": "x"}, {"t": None}, {"t": True}):
            with self.assertRaises(ParamError, msg=bad):
                L["where"](es, bad, "doors")

    def test_a_door_by_side_lays_the_same_house(self):
        a = dict(ATTRS, doors=[{"storey": 0, "t": 0.4, "width": 1}])
        b = dict(ATTRS, doors=[{"storey": 0, "side": 1, "at": 4.0, "width": 1}])    # 16 m along 40 m: 4 m up side 1
        self.assertEqual(plan(a, kit=KIT), plan(b, kit=KIT))

    def test_where_a_gap_would_close(self):
        """A 1.5 m gap in 12 m of wall on a 1.5 m half bay: its middle may lie at 0.3 + 0.75 and
        every 1.5 m on. A 1.7 m gap leaves 10.3 m, which no split closes."""
        self.assertEqual(L["gap_places"](300, 12300, 1500, 3000, 1500, 4050), (4050, 5550))
        self.assertEqual(L["gap_places"](300, 12300, 1500, 3000, 1500, 4500), (4050, 5550))
        self.assertEqual(L["gap_places"](300, 12300, 1700, 3000, 1500, 4050), (None, None))
        self.assertEqual(L["gap_places"](300, 1300, 1500, 3000, 1500, 800), (None, None))
        ring = [(0, 0), (13600, 0), (13600, 9600), (0, 9600)]          # a metre off the module
        kit = dict(self.HALF, door={"id": "frame", "pitch": 1500, "thick": 300, "width": 1500})
        notes = []
        L["wall_plan"](ring, ring, [], kit, "b", "b_s0", [([(4050, -2000), (4050, 0)], 1400, ("door", "b_d0"))],
                       notes=notes)
        (g,) = notes
        self.assertEqual((g["code"], g["side"], g["places"]), ("gap_wide", 0, (None, None)))

    def test_an_open_wall_beside_a_stairwell_says_so(self):
        """A stair 2 m along its side and 4.5 m long leaves the storey above 0.7 m and 5.8 m of
        wall either side of its stairwell: both open, both marked as ending at the stairwell. At
        2.8 m and 5 m long both close."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        attrs = {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": {"side": 2, "at": 2.0, "width": 1, "run": 4.5}}
        out = L["plan"]("hall", ring, attrs, 10.0, GRID, self.HALF)
        self.assertEqual(sorted((o["length"], o["stairwell"]) for o in out["open"]), [(700, True), (5800, True)])
        attrs["stair"] = {"side": 2, "at": 2.8, "width": 1, "run": 5}
        self.assertEqual(L["plan"]("hall", ring, attrs, 10.0, GRID, self.HALF)["open"], [])

    def test_a_kit_number_that_is_no_length_is_said(self):
        rows = [{"id": "bay", "tags": ["role:wall"], "bounds": [-1500, -150, 0, 1500, 150, 3000], "params": {"pitch": 3}},
                {"id": "post", "tags": ["role:corner"], "bounds": [-150, -150, 0, 150, 150, 3000], "params": {}}]
        self.assertEqual(L["kit_pieces"](rows)["corner"]["reach"], None)
        for key, bad in (("reach", "wide"), ("reach", -1), ("reach", True), ("pitch", "3")):
            broken = [dict(r, params=dict(r["params"], **{key: bad})) if r["id"] == "post" else r for r in rows]
            with self.assertRaises(ParamError, msg=(key, bad)) as cm:
                L["kit_pieces"](broken)
            self.assertEqual(cm.exception.key, "kit")

    def test_what_stands_at_an_inside_corner(self):
        """With no inside corner piece a post stands in the notch and takes nothing, even where the
        post says its own reach; a corner that is no post is not put in the notch, and the
        corner with no piece is noted."""
        post = dict(self.HALF, corner={"id": "post", "pitch": 300, "thick": 300, "reach": 300})
        self.assertEqual(L["corner_of"](post, False, 300), (post["corner"], 0))
        self.assertEqual(L["corner_of"](post, True, 300), (post["corner"], 300))
        legged = dict(self.HALF, corner={"id": "ell", "pitch": 1800, "thick": 1800, "reach": 1800})
        self.assertEqual(L["corner_of"](legged, False, 300), (None, 0))
        notes = []
        _, insts, _ = L["wall_plan"](self.ELL, self.ELL, [], legged, "b", "b_s0", notes=notes)
        self.assertEqual(len(insts), 5)
        self.assertEqual([(n["code"], n["outside"], n["at"]) for n in notes if n["code"] == "corner_none"],
                         [("corner_none", False, (5100, 5100))])
        bare = {k: v for k, v in self.HALF.items() if k != "corner"}
        notes = []
        L["wall_plan"](self.ELL, self.ELL, [], bare, "b", "b_s0", notes=notes)
        self.assertEqual(sum(n["code"] == "corner_none" for n in notes), 6)

    def test_a_side_shorter_than_its_corners(self):
        """A 3 m side between two corners that take 1.8 m each: no wall, and the side is noted."""
        ring = [(0, 0), (15600, 0), (15600, 3000), (0, 3000)]
        kit = dict(self.HALF, corner={"id": "ell", "pitch": 1800, "thick": 1800, "reach": 1800})
        notes = []
        runs, _, opens = L["wall_plan"](ring, ring, [], kit, "b", "b_s0", notes=notes)
        self.assertEqual(sorted((n["side"], n["length"], n["takes"]) for n in notes if n["code"] == "side_short"),
                         [(1, 3000, 3600), (3, 3000, 3600)])
        self.assertEqual(opens, [])
        self.assertEqual(sorted(r["count"] for r in runs), [4, 4])

    def test_a_door_in_a_corner_is_refused(self):
        """A door whose 1.5 m opening would reach into the post at its side's start: refused, with
        the stretch its middle may stand on (1.05 to 11.55 m of a 12.6 m side)."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        kit = dict(self.HALF, door={"id": "frame", "pitch": 1500, "thick": 300, "width": 1500})
        attrs = {"storeys": 1, "storey_h": 3.0, "doors": [{"storey": 0, "side": 0, "at": 0.9, "width": 1.4}]}
        with self.assertRaises(ParamError) as cm:
            L["plan"]("hall", ring, attrs, 10.0, GRID, kit)
        self.assertEqual(cm.exception.key, "doors")
        self.assertIn("between 1.05 and 11.55 m", cm.exception.hint)
        attrs["doors"][0]["at"] = 1.05
        self.assertEqual(L["plan"]("hall", ring, attrs, 10.0, GRID, kit)["open"], [])

    def test_an_open_wall_names_the_outline_side(self):
        """On the storey above, the stairwell adds edges to the storey's outline; an open stretch
        still says the building's side, the number `side` takes."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        attrs = {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": {"side": 2, "at": 2.0, "width": 1, "run": 4.5}}
        out = L["plan"]("hall", ring, attrs, 10.0, GRID, self.HALF)
        self.assertEqual({o["side"] for o in out["open"]}, {2})

    def test_wall_faces_out(self):
        out = plan(kit=KIT)
        faces = {r["key"].split("_w")[1]: r["face"] for r in out["runs"] if r["ref"] == "hall_s0"}
        self.assertEqual(faces["0_0"], 180.0)    # the south wall looks south
        self.assertEqual(faces["2_0"], 0.0)


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
    def __init__(self, attrs, ground=10.0):
        self.params, self.grid, self.id, self.for_each_item = {"inner_door": 1}, GRID, "lay", "hall"
        self.out, self.attrs, self.ground, self.said = FakeOut(), attrs, ground, []

    def features(self, select, columns=()):
        fl = _FL({"name": ["hall"], "geometry": [2], "coords": [list(p) for p in HALL]})
        for c in columns:
            has = c in self.attrs
            v = self.attrs.get(c)
            fl[c] = [json.dumps(v) if isinstance(v, (list, dict)) else (v if has else 0)]
            fl[c + "?"] = [1 if has else 0]
        return fl

    def fields(self, names, **kw):
        class A(list):
            name = "height"
        return [A()]

    def eval(self, formula, x, y):
        return [self.ground] * len(x)

    def pack(self, select, columns=()):
        return {"id": ["bay", "corner", "leaf", "strip"],
                "tags": ['["role:wall"]', '["role:corner"]', '["role:door"]', '["role:filler"]'],
                "bounds": [[-500, -150, 0, 500, 150, 3000], [-150, -150, 0, 150, 150, 3000],
                           [-500, -150, 0, 500, 150, 2200], [-50, -150, 0, 50, 150, 3000]],
                "params": ['{"pitch": 1}', "", "", '{"pitch": 0.1}']}

    def _say(sev):
        def say(self, code, subject, message, hint, **kw):
            self.said.append((sev, code, subject, kw.get("data")))
        return say
    error, warning, info = _say("error"), _say("warning"), _say("info")


class Step(unittest.TestCase):
    def test_generate_writes_the_house(self):
        s = FakeStep(dict(ATTRS, kit="building/kit/box", rooms=[{"storey": 0, "shares": [2, 1]}]))
        L["generate"](s)
        self.assertEqual([x[:2] for x in s.said], [("info", "laid")])
        f = s.out.features
        self.assertEqual(sorted(f), ["hall_d0", "hall_f0", "hall_p0", "hall_s0", "hall_s0_r0", "hall_s0_r1",
                                     "hall_s1"])
        self.assertEqual(f["hall_s1"]["level"], 10.3 + 3.0)
        self.assertEqual(f["hall_d0"]["ends_at"], "hall_s0.floor")
        self.assertEqual(f["hall_s0_r0"]["storey"], "hall_s0")
        runs = [k for (sec, k) in s.out.tables if sec == "run"]
        self.assertTrue(runs)
        on = s.out.tables[("run", runs[0])]["on"]
        self.assertIn(on["ref"], ("hall_s0", "hall_s1"))
        self.assertTrue(all(i["surface"].endswith(".floor") for i in s.out.instances.values()))

    def test_a_bad_attribute_is_one_error_and_nothing_returned(self):
        s = FakeStep(dict(ATTRS, doors=[{"storey": 1, "t": 0.4}]))
        L["generate"](s)
        self.assertEqual([x[:3] for x in s.said], [("error", "params", "hall")])
        self.assertEqual(s.out.features, {})


VIA = os.environ.get("VIA_BIN") or shutil.which("via")
WORLD = """name = "house"
extent = [0, 0, 64, 64]
resolution = 1
include = ["building"]

[field.height]
role = "surface"
base = 10

[feature.hall]
kind     = "building"
region   = [[20, 20], [32, 20], [32, 28], [20, 28]]
storeys  = 2
storey_h = 3
porch    = 4
walk_out = 12
stair    = { t = 0.025, width = 1, run = 5 }
doors    = [{ storey = 0, t = 0.4, width = 1 }]
rooms    = [{ storey = 0, shares = [2, 1] }]

[step.lay]
when     = "generate"
worker   = true
for_each = "kind:building"
command  = ["python", "-m", "viasdk", "lib:building/lay.py"]

[step.check]
when    = "lint"
worker  = true
checks  = ["lay"]
command = ["python", "-m", "viasdk", "lib:building/check.py"]
"""


class World(unittest.TestCase):
    """The 6.4 house built by the engine from one building feature: four meetings, every gap 0,
    as the case surfaces/house_meetings asks of the hand-written house (names `lay#hall_*`). Read
    from the build's meets.json: `via query` runs no generate step yet (TODO, From M5 slice 2)."""

    def test_house_meets(self):
        import json
        import tempfile

        self.assertIsNotNone(VIA, "set VIA_BIN")
        lib = os.path.dirname(HERE)
        with tempfile.TemporaryDirectory() as d:
            # Its own config (the step's `python`, this interpreter): never the user's.
            env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(d, ".config"))
            subprocess.run([VIA, "config", "program", "python", sys.executable], cwd=d, check=True,
                           env=env, stdout=subprocess.DEVNULL)
            with open(os.path.join(d, "world.toml"), "w") as f:
                f.write(WORLD)
            subprocess.run([VIA, "trust", "world.toml", "--yes", "--lib", lib], cwd=d, check=True,
                           env=env, stdout=subprocess.DEVNULL)
            subprocess.run([VIA, "build", "world.toml", "--lib", lib, "--out", "build/w"], cwd=d, check=True,
                           env=env)
            got = json.load(open(os.path.join(d, "build", "w", "meets.json")))["meets"]
            got = {k: [m[k] for m in got] for k in ("a", "gap")}
            self.assertEqual(got["a"], ["lay#hall_d0.end", "lay#hall_f0.end", "lay#hall_f0.start", "lay#hall_p0.end"])
            self.assertEqual(got["gap"], [0, 0, 0, 0])


if __name__ == "__main__":
    unittest.main()
