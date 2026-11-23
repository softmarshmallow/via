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
C = runpy.run_path(os.path.join(HERE, "check.py"))
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
        """The landing 1 m before the foot: where the stairwell is cut from the outline (no kit, or
        a corner that is not square), a foot 1.08 m from the corner leaves 0.08 m of wall, which
        no piece closes (M7 close); 1.0 m reaches the corner and 1.3 m leaves 0.3 m. Where the
        wall stands on a footing no stub is left, and the stair may stand anywhere (K4)."""
        slant = [(20000, 20000), (32000, 20000), (33000, 28000), (20000, 28000)]
        for kw in ({}, {"kit": KIT, "ring": slant}):
            with self.assertRaises(ParamError) as e:
                plan({"stair": {"t": 0.027 if not kw else 1080 / L["edges"](slant)[0]["L"] * 12000 / sum(
                    x["L"] for x in L["edges"](slant)), "width": 1, "run": 5}}, **kw)
            self.assertIn("m from the corner", str(e.exception.args))
        for t in (0.025, 0.0325):
            plan({"stair": {"t": t, "width": 1, "run": 5}})
        out = plan({"stair": {"t": 0.027, "width": 1, "run": 5}}, kit=KIT)
        self.assertEqual((out["open"], len(out["footings"])), ([], 1))


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
        its strip (centre 0.9 m in, 1 m wide) starts 0.1 m clear of the wall. The storey above
        keeps off that wall's footing (the south side moved in 0.3 m), and its stairwell is cut
        from there, 0.3 m off the west wall it used to reach."""
        out = plan(kit=KIT)
        (f,) = out["flights"]
        self.assertEqual(f["path"], [(21000, 20900), (26000, 20900)])
        self.assertEqual(f["path"][0][1] - 500 - 20000, 300 + 100)
        self.assertTrue(same_ring(out["storeys"][1]["region"],
                                  [(20000, 20300), (20300, 20300), (20300, 21500), (26000, 21500), (26000, 20300),
                                   (32000, 20300), (32000, 28000), (20000, 28000)]))
        self.assertEqual(out["footings"], [{"key": "hall_s1_footing", "level": 13.3, "side": 0, "region": [
            (20000, 20000), (32000, 20000), (32000, 20300), (20000, 20300)]}])

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
        # A door with its side and no `at` stands on the side's middle (the fix plan's F3); a stair
        # with no `at` says no place.
        self.assertEqual(L["where"](es, {"side": 0}, "doors"), (0, 6300.0))
        self.assertEqual(L["where"](es, {"side": 1}, "doors"), (1, 4800.0))
        with self.assertRaises(ParamError):
            L["where"](es, {"side": 0}, "stair")
        for bad in ({"side": 4, "at": 1}, {"side": 0, "at": 13}, {"side": 4}, {"side": 0, "at": 1, "t": 0.1}, {},
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

    def test_a_stair_no_longer_decides_where_the_wall_above_closes(self):
        """A stair 2 m along its side and 4.5 m long used to leave the storey above 0.7 m and 5.8 m
        of wall either side of its stairwell, both open. The wall over a stairwell now stands on
        its footing, the whole side as on the ground: nothing is open, wherever the stair is."""
        ring = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
        for stair in ({"side": 2, "at": 2.0, "width": 1, "run": 4.5}, {"side": 2, "at": 2.8, "width": 1, "run": 5},
                      {"side": 1, "at": 1.0, "width": 1.2, "run": 6.1}):
            attrs = {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": stair}
            out = L["plan"]("hall", ring, attrs, 10.0, GRID, self.HALF)
            self.assertEqual(out["open"], [], stair)
            self.assertEqual([f["side"] for f in out["footings"]], [stair["side"]])

    def test_a_stair_side_with_a_corner_that_is_not_square_stands_open(self):
        """No footing where the side's neighbour is not square to it: the stairwell is cut from the
        outline as before, and the wall over it is open, marked as ending at the stairwell."""
        ring = [(0, 0), (12600, 0), (13600, 9600), (0, 9600)]
        attrs = {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": {"side": 2, "at": 3.0, "width": 1, "run": 4.5}}
        out = L["plan"]("hall", ring, attrs, 10.0, GRID, self.HALF)
        self.assertEqual(out["footings"], [])
        self.assertTrue(any(o["stairwell"] and o["side"] == 2 for o in out["open"]), out["open"])
        self.assertEqual(len(out["storeys"][1]["region"]), 8)                    # the notch, from the outline

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
        """A side that does not close on the storey above says the building's side, the number
        `side` takes, as on the ground."""
        ring = [(0, 0), (13600, 0), (13600, 9600), (0, 9600)]
        attrs = {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": {"side": 1, "at": 2.0, "width": 1, "run": 4.5}}
        out = L["plan"]("hall", ring, attrs, 10.0, GRID, self.HALF)
        self.assertEqual(sorted((o["side"], o["storey"]) for o in out["open"]), [(0, 0), (0, 1), (2, 0), (2, 1)])

    ROWS = [{"id": "wall", "tags": ["role:wall"], "bounds": [-1500, -150, 0, 1500, 150, 3000], "params": {"pitch": 3}},
            {"id": "wall_arch", "tags": ["role:wall", "variant:arch"], "bounds": [-1500, -150, 0, 1500, 150, 3000],
             "params": {"pitch": 3}},
            {"id": "wall_half", "tags": ["role:filler"], "bounds": [-750, -150, 0, 750, 150, 3000], "params": {"pitch": 1.5}},
            {"id": "wall_window", "tags": ["role:wall", "variant:window"], "bounds": [-1500, -150, 0, 1500, 150, 3000],
             "params": {"pitch": 3}},
            {"id": "a_wide", "tags": ["role:wall", "variant:wide"], "bounds": [-1600, -150, 0, 1600, 150, 3000],
             "params": {"pitch": 3.2}},
            {"id": "corner", "tags": ["role:corner"], "bounds": [-150, -150, 0, 150, 150, 3000], "params": {}}]
    RECT = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]

    def test_the_bay_and_its_variants(self):
        """The bay is the first `role:wall` piece with no `variant:` tag, whatever its id; a variant
        of another size is refused with what differs."""
        kit = L["kit_pieces"](self.ROWS)
        self.assertEqual(kit["wall"]["id"], "wall")
        self.assertEqual({w: v["id"] for w, v in kit["wall"]["variants"].items()},
                         {"arch": "wall_arch", "window": "wall_window"})
        self.assertEqual(kit["wall"]["refused"], {"wide": {
            "id": "a_wide", "why": "pitch 3.2 m for the bay's 3 m, length 3.2 m for the bay's 3 m"}})

    def variant(self, **over):
        return dict({"id": "v", "tags": ["role:wall", "variant:v"], "bounds": [-1500, -150, 0, 1500, 150, 3000],
                     "params": {"pitch": 3}}, **over)

    def test_a_variant_is_the_bays_box_not_only_its_pitch(self):
        """A variant re-exported longer, thicker, taller or off its origin is refused though its
        `params.pitch` still says the bay's: it would stand over its neighbour (the review of K3).
        5 mm is within the fit; 6 mm is not."""
        for bounds, word in (([-1700, -150, 0, 1700, 150, 3000], "length 3.4 m"),
                             ([-1500, -100, 0, 1500, 100, 3000], "thickness 0.2 m"),
                             ([-1500, -150, 0, 1500, 150, 3300], "height 3.3 m"),
                             ([-1500, 50, 0, 1500, 350, 3000], "its box lies 0.2 m off the bay's about its origin"),
                             ([-1500, -150, 0, 1500, 156, 3000], "thickness 0.306 m")):
            kit = L["kit_pieces"](self.ROWS[:1] + [self.variant(bounds=bounds)])
            self.assertEqual(kit["wall"]["variants"], {}, bounds)
            self.assertIn(word, kit["wall"]["refused"]["v"]["why"])
        kit = L["kit_pieces"](self.ROWS[:1] + [self.variant(bounds=[-1500, -150, 0, 1500, 155, 3000])])
        self.assertEqual(list(kit["wall"]["variants"]), ["v"])
        picks = L["picks_of"]({"fill": [{"sides": [0], "pattern": ["v"]}]})
        long = L["kit_pieces"](self.ROWS + [self.variant(bounds=[-1700, -150, 0, 1700, 150, 3000])])
        runs, _, _ = L["wall_plan"](self.RECT, self.RECT, [], long, "b", "b_s0", picks=picks, name="hall")
        self.assertEqual(self.laid(runs, 0), [("b_w0_0", "wall", 4)])

    def test_the_words_of_a_kit(self):
        """Of two pieces with one word the first that is the bay's size is taken; a piece answers
        to each of its `variant:` tags; `wall` is no variant's word; in a kit of variants alone the
        first is the bay and answers to its own word; an untagged piece is its role's, whatever
        its id."""
        bad = self.variant(id="win_a", tags=["role:wall", "variant:window"], bounds=[-1600, -150, 0, 1600, 150, 3000])
        good = self.variant(id="win_b", tags=["role:wall", "variant:window", "variant:glass"])
        kit = L["kit_pieces"](self.ROWS[:1] + [bad, good])
        self.assertEqual({w: v["id"] for w, v in kit["wall"]["variants"].items()}, {"window": "win_b", "glass": "win_b"})
        self.assertEqual(kit["wall"]["refused"], {})
        kit = L["kit_pieces"](self.ROWS[:1] + [self.variant(tags=["role:wall", "variant:wall"])])
        self.assertEqual((kit["wall"]["variants"], list(kit["wall"]["refused"])), ({}, ["wall"]))
        only = L["kit_pieces"]([self.variant(id="arch", tags=["role:wall", "variant:arch"]),
                                self.variant(id="bow", tags=["role:wall", "variant:bow"])])
        self.assertEqual((only["wall"]["id"], sorted(only["wall"]["variants"])), ("arch", ["arch", "bow"]))
        kit = L["kit_pieces"](self.ROWS + [self.variant(id="a_half_glass", tags=["role:filler", "variant:glass"],
                                                        bounds=[-750, -150, 0, 750, 150, 3000])])
        self.assertEqual(kit["filler"]["id"], "wall_half")

    def test_no_choice_lays_the_bay_as_before(self):
        kit = L["kit_pieces"](self.ROWS)
        plain = L["kit_pieces"]([r for r in self.ROWS if not any(t.startswith("variant:") for t in r["tags"])])
        a = L["wall_plan"](self.RECT, self.RECT, [], kit, "b", "b_s0")
        b = L["wall_plan"](self.RECT, self.RECT, [], plain, "b", "b_s0")
        self.assertEqual(a, b)

    def laid(self, runs, edge):
        """`[(key, asset, count)]` of one side's runs in order along it; for a run that names each
        station's piece (`assets`), the list of them in place of the one asset."""
        return [(r["key"], r.get("assets") or r["asset"], r["count"]) for r in sorted(runs, key=lambda r: r["span"][0])
                if r["key"].startswith(f"b_w{edge}_")]

    @staticmethod
    def stations(runs, edge, total):
        """Where the bays of one side stand, mm along the ring, as rules.md 5.1 places a run's
        members (`ends = "half"`): `[(station, asset)]` in order."""
        out = []
        for r in runs:
            if r["key"].startswith(f"b_w{edge}_") and not r["key"].endswith("f"):
                a, b = round(r["span"][0] * total), round(r["span"][1] * total)
                out += [(a + (2 * k + 1) * (b - a) // (2 * r["count"]), (r.get("assets") or [r["asset"]] * r["count"])[k])
                        for k in range(r["count"])]
        return sorted(out)

    def test_a_fill_lays_its_pattern_in_one_run(self):
        """`["window", "wall"]` along the 12 m south side: one run, named as the stretch always was,
        that names each station's piece (R68). A side of one piece still names the one asset."""
        kit = L["kit_pieces"](self.ROWS)
        picks = L["picks_of"]({"fill": [{"sides": [0], "pattern": ["window", "wall"]}]})
        runs, _, opens = L["wall_plan"](self.RECT, self.RECT, [], kit, "b", "b_s0", picks=picks, name="hall")
        self.assertEqual(opens, [])
        self.assertEqual(self.laid(runs, 0), [("b_w0_0", ["wall_window", "wall", "wall_window", "wall"], 4)])
        (south,) = [r for r in runs if r["key"] == "b_w0_0"]
        self.assertNotIn("asset", south)
        self.assertAlmostEqual((south["span"][1] - south["span"][0]) * 44400, 12000, delta=0.01)
        self.assertEqual(self.laid(runs, 1), [("b_w1_0", "wall", 3)])           # the other sides as before
        self.assertNotIn("assets", [r for r in runs if r["key"] == "b_w1_0"][0])
        plain, _, _ = L["wall_plan"](self.RECT, self.RECT, [], kit, "b", "b_s0")
        same = lambda r: {k: v for k, v in r.items() if k not in ("asset", "assets")}      # noqa: E731
        self.assertEqual([same(r) for r in runs], [same(r) for r in plain])    # nothing but the pieces differs
        one = L["picks_of"]({"fill": [{"sides": [0], "pattern": ["window"]}]})
        runs, _, _ = L["wall_plan"](self.RECT, self.RECT, [], kit, "b", "b_s0", picks=one, name="hall")
        self.assertEqual(self.laid(runs, 0), [("b_w0_0", "wall_window", 4)])    # all one variant: the one asset

    def test_a_mixed_run_stands_where_the_one_run_stood(self):
        """A side 4 mm longer than its pieces closes with slack, which the run spreads over its
        bays: with a variant in it the run is the same run, so no bay moves. With half bays after
        them, the bays end where the fillers start."""
        kit = L["kit_pieces"](self.ROWS)
        slack = [(0, 0), (12604, 0), (12604, 9600), (0, 9600)]
        total = 2 * (12604 + 9600)
        picks = L["picks_of"]({"fill": [{"sides": [0], "pattern": ["window", "wall"]}]})
        one, _, _ = L["wall_plan"](slack, slack, [], kit, "b", "b_s0")
        mixed, _, opens = L["wall_plan"](slack, slack, [], kit, "b", "b_s0", picks=picks, name="hall")
        self.assertEqual(opens, [])
        a, b = self.stations(one, 0, total), self.stations(mixed, 0, total)
        self.assertEqual([x[0] for x in a], [x[0] for x in b])
        self.assertEqual([x[1] for x in b], ["wall_window", "wall", "wall_window", "wall"])
        (run,) = [r for r in mixed if r["key"] == "b_w0_0"]
        self.assertAlmostEqual(run["span"][1] * total, 12604 - 300, delta=0.001)       # the run reaches the corner
        halves = [(0, 0), (14100, 0), (14100, 9600), (0, 9600)]                    # 13.5 m: four bays and a half bay
        runs, _, _ = L["wall_plan"](halves, halves, [], kit, "b", "b_s0", picks=picks, name="hall")
        (bays,) = [r for r in runs if r["key"] == "b_w0_0"]
        (fill,) = [r for r in runs if r["key"] == "b_w0_0f"]
        self.assertAlmostEqual(bays["span"][1], fill["span"][0], places=9)
        self.assertAlmostEqual(fill["span"][0] * 2 * (14100 + 9600), 300 + 12000, delta=0.001)

    def test_a_fill_names_its_storeys_and_sides_and_a_later_one_lies_over(self):
        bv = L["bay_variants"]
        up = L["picks_of"]({"fill": [{"storeys": [1], "pattern": ["window"]}]})
        self.assertEqual((bv(3, 300, 3000, 0, 0, up, "hall"), bv(3, 300, 3000, 0, 1, up, "hall")),
                         ([None] * 3, ["window"] * 3))
        south = L["picks_of"]({"fill": [{"pattern": ["arch"]}, {"sides": [0], "pattern": ["window", "wall"]}]})
        self.assertEqual(bv(3, 300, 3000, 0, 0, south, "hall"), ["window", None, "window"])
        self.assertEqual(bv(3, 300, 3000, 1, 0, south, "hall"), ["arch"] * 3)

    def test_a_choice_is_made_from_the_place(self):
        """A list of words is a choice by place: the same every time; another for another building,
        storey or side; and for one bay the same whatever else the stretch holds, unless a bay
        beside it with the same word came or went."""
        bv, kit = L["bay_variants"], L["kit_pieces"](self.ROWS)
        picks = L["picks_of"]({"fill": [{"pattern": [["window", "arch", "wall"]]}]})
        a = bv(20, 300, 3000, 0, 0, picks, "hall")
        self.assertEqual(a, bv(20, 300, 3000, 0, 0, picks, "hall"))
        self.assertEqual({x for x in a}, {"window", "arch", None})
        for other in (bv(20, 300, 3000, 0, 0, picks, "annex"), bv(20, 300, 3000, 0, 1, picks, "hall"),
                      bv(20, 300, 3000, 1, 0, picks, "hall")):
            self.assertNotEqual(a, other)
        whole = 0
        for i in range(200):
            name = f"house{i}"
            full, less = bv(20, 300, 3000, 0, 0, picks, name), bv(19, 3300, 3000, 0, 0, picks, name)
            self.assertTrue(all(x != y for x, y in zip(full, full[1:])), name)       # no two that touch alike
            diff = [x != y for x, y in zip(full[1:], less)]
            self.assertEqual(diff, sorted(diff, reverse=True), name)                 # only next to what went
            self.assertLessEqual(sum(diff), 6, name)
            whole += not any(diff)
        self.assertGreater(whole, 100)
        ring = [(0, 0), (60600, 0), (60600, 9600), (0, 9600)]              # twenty bays along the south side
        runs, _, opens = L["wall_plan"](ring, ring, [], kit, "b", "b_s0", picks=picks, name="hall")
        self.assertEqual(opens, [])
        ((key, pieces, count),) = self.laid(runs, 0)
        self.assertEqual((key, count, len(pieces)), ("b_w0_0", 20, 20))
        self.assertTrue(all(x != y for x, y in zip(pieces, pieces[1:])))   # no two that touch alike

    def test_two_pieces_go_in_turn_and_words_of_one_piece_are_one(self):
        """Two pieces can only alternate, which is done by the bay's number along the side, so it
        too holds for a bay that does not move. A word the kit cannot give lays the bay, as `wall`
        does: the two are one piece, and the choice is among pieces."""
        bv, variants = L["bay_variants"], L["kit_pieces"](self.ROWS)["wall"]["variants"]
        two = L["picks_of"]({"fill": [{"pattern": [["window", "wall"]]}]})
        for name in ("hall", "annex", "mill"):
            a = bv(20, 300, 3000, 0, 0, two, name, variants)
            self.assertTrue(all(x != y for x, y in zip(a, a[1:])))
            self.assertEqual(a[1:], bv(19, 3300, 3000, 0, 0, two, name, variants))
            self.assertEqual(a, bv(20, 1500, 3000, 0, 0, two, name, variants))    # a start on a half pitch too
        same = L["picks_of"]({"fill": [{"pattern": [["wall", "glass", "wide", "window"]]}]})
        for name in (f"house{i}" for i in range(50)):
            a = [w if w in variants else None for w in bv(20, 300, 3000, 0, 0, same, name, variants)]
            self.assertTrue(all(x != y for x, y in zip(a, a[1:])), name)

    def test_a_choice_leaves_out_the_fixed_word_beside_it(self):
        bv, variants = L["bay_variants"], L["kit_pieces"](self.ROWS)["wall"]["variants"]
        picks = L["picks_of"]({"fill": [{"pattern": [["window", "arch"], "window"]}]})
        for name in ("hall", "annex", "mill", "quay"):
            self.assertEqual(bv(6, 300, 3000, 0, 0, picks, name, variants), ["arch", "window"] * 3)
        three = L["picks_of"]({"fill": [{"pattern": [["window", "arch", "wall"], "window"]}]})
        for name in (f"house{i}" for i in range(50)):
            a = bv(20, 300, 3000, 0, 0, three, name, variants)
            self.assertTrue(all(x != y for x, y in zip(a, a[1:])), name)

    def test_one_bay_by_a_point_along_its_side(self):
        """`bays`: the bay that holds 7 m along the south side (the third: 6.3 to 9.3 m) takes the
        window, over what a fill gave it. A point in a corner's reach names no bay and is noted,
        at the point asked for."""
        kit = L["kit_pieces"](self.ROWS)
        attrs = {"storeys": 1, "storey_h": 3.0, "doors": [],
                 "fill": [{"sides": [0], "pattern": ["arch"]}],
                 "bays": [{"storey": 0, "side": 0, "at": 7.0, "variant": "window"},
                          {"storey": 0, "side": 1, "at": 0.1, "variant": "window"}]}
        out = L["plan"]("hall", self.RECT, attrs, 10.0, GRID, kit)
        south = [(r["key"], r["assets"]) for r in out["runs"] if r["key"].startswith("hall_s0_w0_")]
        self.assertEqual(south, [("hall_s0_w0_0", ["wall_arch", "wall_arch", "wall_window", "wall_arch"])])
        self.assertEqual([(n["code"], n["index"], n["side"], tuple(n["at"])) for n in out["notes"]],
                         [("bay_none", 1, 1, (12600, 100))])

    def test_the_bay_that_holds_the_point(self):
        """Not the nearest start: 9.2 m is still the third bay (6.3 to 9.3 m). A point on a joint is
        the bay after it, whatever the float; before the first bay or past the last is none.
        `bays` names its storey and its side, and `wall` there puts the bay back."""
        bv = L["bay_variants"]

        def at(x, storey=0, side=0, n=4, first=300, pitch=3000, variant="window", fill=()):
            picks = L["picks_of"]({"fill": list(fill), "bays": [{"storey": 1, "side": 2, "at": x, "variant": variant}]})
            return bv(n, first, pitch, side, storey, picks, "hall"), picks["bays"][0]["hit"]
        self.assertEqual(at(9.2, 1, 2), ([None, None, "window", None], True))
        self.assertEqual(at(9.3, 1, 2), ([None, None, None, "window"], True))
        self.assertEqual(at(0.3, 1, 2), (["window", None, None, None], True))
        self.assertEqual(at(0.299, 1, 2), ([None] * 4, False))
        self.assertEqual(at(12.3, 1, 2), ([None] * 4, False))
        self.assertEqual(at(-1, 1, 2), ([None] * 4, False))
        self.assertEqual(at(5.0, 0, 2), ([None] * 4, False))                 # another storey
        self.assertEqual(at(5.0, 1, 0), ([None] * 4, False))                 # another side
        self.assertEqual(at(32.3, 1, 2, n=40, pitch=1000)[0].index("window"), 32)
        self.assertEqual(at(513.3, 1, 2, n=400, pitch=1500)[0].index("window"), 342)
        self.assertEqual(at(5.0, 1, 2, variant="wall", fill=[{"pattern": ["arch"]}]),
                         (["arch", None, "arch", "arch"], True))

    def test_a_variant_the_kit_cannot_give(self):
        """Asked for once and noted once per word, on however many storeys and sides: the bay is
        laid. A word in a choice is noted though no bay took it."""
        kit = L["kit_pieces"](self.ROWS)
        picks = L["picks_of"]({"fill": [{"sides": [0], "pattern": ["wide", "glass"]}],
                               "bays": [{"side": 3, "at": 4, "variant": "bow"}]})
        notes = []
        for storey in ("b_s0", "b_s1"):
            runs, _, _ = L["wall_plan"](self.RECT, self.RECT, [], kit, "b", storey, picks=picks, name="hall", notes=notes)
            self.assertEqual(self.laid(runs, 0), [("b_w0_0", "wall", 4)])
        self.assertEqual(sorted((n["variant"], (n["refused"] or {}).get("id")) for n in notes),
                         [("bow", None), ("glass", None), ("wide", "a_wide")])

    def test_picks_are_checked(self):
        nan = float("nan")
        for attrs in ({"bays": [{"storey": 0, "side": 0, "at": 1}]}, {"bays": [{"side": 0.5, "at": 1, "variant": "w"}]},
                      {"bays": "window"}, {"fill": [{"pattern": []}]}, {"fill": [{"pattern": [3]}]},
                      {"fill": [{"sides": "south", "pattern": ["wall"]}]}, {"fill": [{"pattern": [[]]}]},
                      {"fill": [{"storey": 1, "side": 0, "pattern": ["window"]}]},         # the keys `bays` takes
                      {"fill": [{"pattern": ["window"], "seed": 3}]},
                      {"bays": [{"side": 0, "at": 1, "variant": "w", "storeys": [0]}]},
                      {"bays": [{"side": 0, "at": nan, "variant": "w"}]},
                      {"bays": [{"side": 0, "at": float("inf"), "variant": "w"}]},
                      {"bays": [{"side": 0, "at": 1e306, "variant": "w"}]},
                      {"bays": [{"side": 0, "at": True, "variant": "w"}]},
                      {"bays": [{"side": 0, "at": 1, "variant": ""}]},
                      {"bays": [{"side": True, "at": 1, "variant": "w"}]},
                      {"fill": [{"pattern": [""]}]}, {"fill": [{"pattern": [["window", "window", "vent"]]}]},
                      {"fill": [{"storeys": [], "pattern": ["window"]}]}, {"fill": [{"sides": [1.5], "pattern": ["w"]}]},
                      {"fill": 0}, {"fill": ""}, {"fill": {}}, {"bays": False}, {"fill": ["window"]}, {"bays": [3]}):
            with self.assertRaises(ParamError, msg=attrs):
                L["picks_of"](attrs)
        self.assertEqual(L["picks_of"]({}), {"bays": [], "fill": []})
        self.assertEqual(L["picks_of"]({"bays": None, "fill": []}), {"bays": [], "fill": []})
        with self.assertRaises(ParamError) as c:
            L["picks_of"]({"fill": 0})
        self.assertEqual(c.exception.key, "fill")

    def test_picks_name_what_the_building_has(self):
        """A storey or a side the building has not is refused, not laid nowhere in silence; so are
        `fill` and `bays` with no kit."""
        kit = L["kit_pieces"](self.ROWS)
        base = {"storeys": 1, "storey_h": 3.0, "doors": []}
        for more in ({"fill": [{"storeys": [1], "pattern": ["window"]}]}, {"fill": [{"sides": [4], "pattern": ["window"]}]},
                     {"bays": [{"storey": 1, "side": 0, "at": 5, "variant": "window"}]},
                     {"bays": [{"side": 9, "at": 5, "variant": "window"}]}):
            with self.assertRaises(ParamError, msg=more):
                L["plan"]("hall", self.RECT, dict(base, **more), 10.0, GRID, kit)
        with self.assertRaises(ParamError):
            L["plan"]("hall", self.RECT, dict(base, fill=[{"pattern": ["window"]}]), 10.0, GRID, None)
        L["plan"]("hall", self.RECT, dict(base, fill=[{"storeys": [0], "sides": [3], "pattern": ["window"]}]), 10.0, GRID, kit)

    def test_a_vertex_in_a_straight_side_starts_the_next_side(self):
        """Sides are counted as doors count them: a point in the middle of a straight wall ends one
        side and starts the next, so `fill` and `bays` reach the second half by its own number."""
        ring = [(0, 0), (6300, 0), (12600, 0), (12600, 9600), (0, 9600)]
        stretches, _ = L["wall_stretches"](ring, ring)
        self.assertEqual([(s["edge"], s["side"], s["off"]) for s in stretches],
                         [(0, 0, 0), (1, 1, 0), (2, 2, 0), (3, 3, 0), (4, 4, 0)])

    def test_every_attribute_has_its_default(self):
        """An attribute `lay` reads that kinds.toml does not declare is never handed to the step."""
        import tomllib
        with open(os.path.join(HERE, "kinds.toml"), "rb") as f:
            defaults = tomllib.load(f)["kind"]["building"]["defaults"]
        self.assertEqual(sorted(set(L["ATTRS"]) - set(defaults)), [])

    def test_wall_faces_out(self):
        out = plan(kit=KIT)
        faces = {r["key"].split("_w")[1]: r["face"] for r in out["runs"] if r["ref"] == "hall_s0"}
        self.assertEqual(faces["0_0"], 180.0)    # the south wall looks south
        self.assertEqual(faces["2_0"], 0.0)


def stations(run, ring):
    """Where the core stands a run's members, mm along `ring` (rules.md 5.1, `ends = "half"`)."""
    P = round(sum(math.dist(p, q) for p, q in zip(ring, ring[1:] + ring[:1])))
    a, b = round(run["span"][0] * P), round(run["span"][1] * P)
    return [a + (2 * k + 1) * (b - a) // (2 * run["count"]) for k in range(run["count"])]


def point_at(ring, s, offset):
    """The point `s` mm along `ring`, `offset` mm to the left."""
    c = 0.0
    for p, q in zip(ring, ring[1:] + ring[:1]):
        L_ = math.dist(p, q)
        if s <= c + L_:
            e = ((q[0] - p[0]) / L_, (q[1] - p[1]) / L_)
            return (round(p[0] + e[0] * (s - c) - e[1] * offset), round(p[1] + e[1] * (s - c) + e[0] * offset))
        c += L_
    raise AssertionError(s)


class Upward(unittest.TestCase):
    """K4: storeys of two heights, pieces by storey, and the wall over a stairwell on its footing."""

    def row(id_, role, size, high=3000, tags=(), **params):
        w, d = size
        return {"id": id_, "tags": [f"role:{role}", *tags], "bounds": [-w // 2, -d // 2, 0, w // 2, d // 2, high],
                "params": params}
    ROWS = [row("wall", "wall", (3000, 300), pitch=3), row("half", "filler", (1500, 300), pitch=1.5),
            row("post", "corner", (300, 300)), row("door", "door", (1500, 300), pitch=1.5),
            row("window", "wall", (3000, 300), tags=("variant:window",), pitch=3),
            row("g_wall", "wall", (3000, 300), 4000, ("storey:ground",), pitch=3),
            row("g_half", "filler", (1500, 300), 4000, ("storey:ground",), pitch=1.5),
            row("g_post", "corner", (300, 300), 4000, ("storey:ground",)),
            row("g_door", "door", (1500, 300), 4000, ("storey:ground",), pitch=1.5),
            row("g_shop", "wall", (3000, 300), 4000, ("storey:ground", "variant:shop"), pitch=3)]
    row = staticmethod(row)
    RECT = [(0, 0), (12600, 0), (12600, 9600), (0, 9600)]
    STAIR = {"side": 2, "at": 2.8, "width": 1, "run": 5}

    def kits(self, rows=None):
        rows = self.ROWS if rows is None else rows
        return lambda word, height: L["kit_pieces"](rows, word, height)

    def plan(self, ring=None, **attrs):
        a = {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": self.STAIR}
        a.update(attrs)
        return L["plan"]("hall", ring or self.RECT, a, 10.0, GRID, None, kits=self.kits())

    def test_storeys_of_two_heights(self):
        """The ground storey 4 m, the top 3.5 m, the one between at `storey_h`: each level is the
        heights below it, the flights climb between them, and the eaves are the top's own height
        up. With neither said, nothing changes."""
        self.assertEqual(L["storey_kinds"](1, 3.0), [("ground", 3.0)])
        self.assertEqual(L["storey_kinds"](2, 3.0, 4.0), [("ground", 4.0), ("top", 3.0)])
        self.assertEqual(L["storey_kinds"](4, 3.0, 4.0, 3.5), [("ground", 4.0), ("upper", 3.0), ("upper", 3.0), ("top", 3.5)])
        attrs = dict(ATTRS, storeys=3, stair={"t": 0.025, "width": 1, "run": 5}, roof="garden")
        out = L["plan"]("hall", HALL, dict(attrs, ground_h=4.0, top_h=3.5), 10.0, GRID)
        self.assertEqual([s["level"] for s in out["storeys"]], [10.3, 14.3, 17.3])
        self.assertEqual([(f["z0"], f["z1"]) for f in out["flights"]], [(10.3, 14.3), (14.3, 17.3)])
        self.assertEqual(out["roof"]["eaves"], 20.8)
        same = L["plan"]("hall", HALL, attrs, 10.0, GRID)
        self.assertEqual(same, L["plan"]("hall", HALL, dict(attrs, ground_h=0, top_h=0), 10.0, GRID))
        self.assertEqual([s["level"] for s in same["storeys"]], [10.3, 13.3, 16.3])
        for bad in ({"ground_h": 0.2}, {"top_h": -1}, {"ground_h": "tall"}):
            with self.assertRaises(ParamError, msg=bad):
                L["plan"]("hall", HALL, dict(attrs, **bad), 10.0, GRID)

    def test_a_storey_takes_the_pieces_of_its_height_and_its_word(self):
        ids = lambda kit: {r: kit[r]["id"] for r in ("wall", "filler", "corner", "door")}       # noqa: E731
        kp = L["kit_pieces"]
        tall = {"wall": "g_wall", "filler": "g_half", "corner": "g_post", "door": "g_door"}
        plain = {"wall": "wall", "filler": "half", "corner": "post", "door": "door"}
        self.assertEqual(ids(kp(self.ROWS, "ground", 4000)), tall)
        self.assertEqual(ids(kp(self.ROWS, "ground", 3000)), plain)         # a 3 m ground storey: the 3 m pieces
        self.assertEqual(ids(kp(self.ROWS, "upper", 3000)), plain)
        self.assertEqual(ids(kp(self.ROWS, "top", 4000)), plain)            # tagged for another storey: left out
        self.assertEqual(ids(kp(self.ROWS)), plain)                         # no word: no tagged piece
        self.assertEqual(sorted(kp(self.ROWS, "ground", 4000)["wall"]["variants"]), ["shop"])
        self.assertEqual(sorted(kp(self.ROWS, "ground", 3000)["wall"]["variants"]), ["window"])
        # of one height, the piece tagged for the storey is read before the untagged one
        rows = self.ROWS + [self.row("z_base", "wall", (3000, 300), 3000, ("storey:ground",), pitch=3)]
        self.assertEqual(kp(rows, "ground", 3000)["wall"]["id"], "z_base")
        self.assertEqual(kp(rows, "upper", 3000)["wall"]["id"], "wall")
        # no piece of the storey's height: the nearest stay (and `check` says `kit_height`)
        self.assertEqual(kp(self.ROWS[:5], "ground", 4000)["wall"]["id"], "wall")
        self.assertEqual(ids(kp(self.ROWS, "ground", 5000)), tall)
        # a bay re-exported 0.3 m too tall is still the bay of a 3 m ground storey, not the 4 m one
        high = [dict(r, bounds=r["bounds"][:5] + [3300]) if r["id"] == "wall" else r for r in self.ROWS]
        self.assertEqual(ids(kp(high, "ground", 3000)), plain)
        self.assertEqual(kp(high, "ground", 3000)["wall"]["high"], 3300)

    def test_a_tall_ground_storey_is_laid_with_its_own_pieces(self):
        out = self.plan(storeys=3, ground_h=4.0, stair={"side": 2, "at": 1.3, "width": 1.3, "run": 6.5},
                        doors=[{"storey": 0, "side": 0, "at": 4.05, "width": 1.4}])
        by = lambda k: {r["asset"] for r in out["runs"] if r["key"].startswith(f"hall_s{k}_")}   # noqa: E731
        self.assertEqual((by(0), by(1), by(2)), ({"g_wall", "g_half"}, {"wall"}, {"wall"}))
        posts = {i["key"]: i["asset"] for i in out["instances"]}
        self.assertEqual((posts["hall_s0_c0"], posts["hall_s1_c0"], posts["hall_s0_door0"]), ("g_post", "post", "g_door"))
        self.assertEqual(out["open"], [])
        low = self.plan(doors=[{"storey": 0, "side": 0, "at": 4.05, "width": 1.4}])            # a 3 m ground storey
        self.assertEqual({r["asset"] for r in low["runs"]}, {"wall", "half"})
        self.assertEqual(low["notes"], [])

    def test_walls_of_two_thicknesses_do_not_stack(self):
        rows = [dict(r, bounds=[r["bounds"][0], -200, 0, r["bounds"][3], 200, 4000]) if r["id"] == "g_wall" else r
                for r in self.ROWS]
        with self.assertRaises(ParamError) as c:
            L["plan"]("hall", self.RECT, {"storeys": 2, "storey_h": 3.0, "ground_h": 4.0, "doors": [], "stair": self.STAIR},
                      10.0, GRID, None, kits=self.kits(rows))
        self.assertEqual(c.exception.key, "kit")

    def test_the_footing_and_the_storey_share_out_the_outline(self):
        """The footing is the stair side's strip, a wall thick, its first edge the side itself. The
        storey keeps off it and off its stairwell, which stops a wall's thickness short of the
        walls beside it: storey, footing and stairwell make up the outline's area."""
        out = self.plan()
        (f,) = out["footings"]
        self.assertEqual((f["key"], f["level"], f["side"]), ("hall_s1_footing", 13.3, 2))
        self.assertEqual(f["region"], [(12600, 9600), (0, 9600), (0, 9300), (12600, 9300)])
        storey = out["storeys"][1]["region"]
        well = 6000 * 1200                                   # 1 m before the foot to the head; one lane and its clear
        self.assertAlmostEqual(area(storey) + area(f["region"]) + well, area(self.RECT))
        self.assertTrue(same_ring(storey, [(0, 0), (12600, 0), (12600, 9300), (10800, 9300), (10800, 8100),
                                           (4800, 8100), (4800, 9300), (0, 9300)]))
        tight = self.plan(stair={"side": 2, "at": 1.0, "width": 1, "run": 5})["storeys"][1]["region"]
        self.assertIn((12300, 9300), tight)                  # a stairwell that reached the corner now stops 0.3 m off it
        self.assertNotIn((12600, 9300), [p for p in tight if p[1] < 9300])
        flight = out["flights"][0]
        self.assertTrue(L["inside"](storey, flight["path"][-1]))
        self.assertTrue(L["inside"](out["storeys"][0]["region"], flight["path"][0]))

    def test_every_side_above_closes_as_on_the_ground(self):
        """The storey over the stairwell has the ground storey's walls: the same runs by name, the
        same pieces, each bay over the bay below. The stair's side stands on the footing, the
        others on the storey; the two corners at the stair side's ends on the footing."""
        out = self.plan()
        self.assertEqual(out["open"], [])
        ground = {r["key"][len("hall_s0"):]: r for r in out["runs"] if r["key"].startswith("hall_s0_")}
        above = {r["key"][len("hall_s1"):]: r for r in out["runs"] if r["key"].startswith("hall_s1_")}
        self.assertEqual(sorted(ground), sorted(above))
        rings = {"hall_s0": out["storeys"][0]["region"], "hall_s1": out["storeys"][1]["region"],
                 "hall_s1_footing": out["footings"][0]["region"]}
        for key, g in ground.items():
            a = above[key]
            self.assertEqual((a["asset"], a["count"], a["face"], a["offset"]), (g["asset"], g["count"], g["face"], g["offset"]))
            self.assertEqual(a["ref"], "hall_s1_footing" if key.startswith("_w2_") else "hall_s1", key)
            here = [point_at(rings[a["ref"]], s, a["offset"] * 1000) for s in stations(a, rings[a["ref"]])]
            below = [point_at(rings["hall_s0"], s, g["offset"] * 1000) for s in stations(g, rings["hall_s0"])]
            self.assertEqual(here, below, key)
        self.assertEqual(sum(r["count"] for r in above.values() if r["asset"] == "wall"), 4 + 3 + 4 + 3)
        on = {i["key"]: i["surface"] for i in out["instances"] if i["key"].startswith("hall_s1_c")}
        self.assertEqual(on, {"hall_s1_c0": "hall_s1.floor", "hall_s1_c1": "hall_s1.floor",
                              "hall_s1_c2": "hall_s1_footing.top", "hall_s1_c3": "hall_s1_footing.top"})
        at = {i["key"][len("hall_s1"):]: i["at"] for i in out["instances"] if i["key"].startswith("hall_s1_c")}
        self.assertEqual(at, {i["key"][len("hall_s0"):]: i["at"] for i in out["instances"] if i["key"].startswith("hall_s0_c")})

    def test_three_storeys_close_over_both_stairwells(self):
        """Two flights, the second back over the first and two lanes deep: each storey above has
        its footing and its own stairwell, and no wall is open (15 m was, in examples/kit)."""
        out = self.plan(storeys=3, stair={"side": 2, "at": 1.3, "width": 1.3, "run": 6.5})
        self.assertEqual(out["open"], [])
        self.assertEqual([(f["key"], f["level"]) for f in out["footings"]],
                         [("hall_s1_footing", 13.3), ("hall_s2_footing", 16.3)])
        one, two = out["storeys"][1]["region"], out["storeys"][2]["region"]
        strip = 12600 * 300
        self.assertAlmostEqual(area(self.RECT) - area(one) - strip, 7500 * 1500)       # 1 m before the foot to the head
        self.assertAlmostEqual(area(self.RECT) - area(two) - strip, 7500 * 2900)       # the foot to 1 m past the head
        for k in (1, 2):
            walls = [r for r in out["runs"] if r["key"].startswith(f"hall_s{k}_w")]
            self.assertEqual(sum(r["count"] for r in walls), 14)
            self.assertEqual({r["ref"] for r in walls if "_w2_" in r["key"]}, {f"hall_s{k}_footing"})
        for f in out["flights"]:
            lo, hi = int(f["starts_at"][6]), int(f["ends_at"][6])
            self.assertTrue(L["inside"](out["storeys"][lo]["region"], f["path"][0]), f["key"])
            self.assertTrue(L["inside"](out["storeys"][hi]["region"], f["path"][-1]), f["key"])

    def test_a_stair_side_that_ends_at_an_inside_corner(self):
        """An L whose stair climbs the side between an outside and an inside corner: the footing
        is that side's strip, the storey's ring runs on past the inside corner by the wall's
        thickness, the inside corner piece stands on the floor, and every side closes."""
        ell = [(0, 0), (9600, 0), (9600, 5100), (5100, 5100), (5100, 9600), (0, 9600)]
        rows = self.ROWS + [{"id": "corner_in", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000],
                             "params": {"reach": 1.2}}]
        out = L["plan"]("hall", ell, {"storeys": 2, "storey_h": 3.0, "doors": [],
                                      "stair": {"side": 2, "at": 1.0, "width": 1, "run": 3.2}},
                        10.0, GRID, None, kits=self.kits(rows))
        self.assertEqual(out["open"], [])
        (f,) = out["footings"]
        self.assertEqual(f["region"], [(9600, 5100), (5100, 5100), (5100, 4800), (9600, 4800)])
        storey = out["storeys"][1]["region"]
        self.assertIn((5100, 4800), storey)                    # the side before the inside corner, a wall's thickness longer
        self.assertAlmostEqual(area(storey) + area(f["region"]) + (4200 - 300) * 1200, area(ell))
        on = {i["asset"]: i["surface"] for i in out["instances"] if i["key"].startswith("hall_s1_c")}
        self.assertEqual(on["corner_in"], "hall_s1.floor")
        self.assertEqual({r["ref"] for r in out["runs"] if r["key"].startswith("hall_s1_w2_")}, {"hall_s1_footing"})
        ground = sorted((r["key"][7:], r["asset"], r["count"]) for r in out["runs"] if r["key"].startswith("hall_s0_"))
        above = sorted((r["key"][7:], r["asset"], r["count"]) for r in out["runs"] if r["key"].startswith("hall_s1_"))
        self.assertEqual(ground, above)

    @staticmethod
    def turned(ring, degrees, about=(6000, 5000)):
        c, s = math.cos(math.radians(degrees)), math.sin(math.radians(degrees))
        return [(round(about[0] + (x - about[0]) * c - (y - about[1]) * s),
                 round(about[1] + (x - about[0]) * s + (y - about[1]) * c)) for x, y in ring]

    def bays(self, out, k, rings=None):
        """Where the core stands the wall bays of storey `k`: `{run key less the storey: [points]}`."""
        rings = {s["key"]: s["region"] for s in out["storeys"]}
        rings.update({f["key"]: f["region"] for f in out["footings"]})
        got = {}
        for r in out["runs"]:
            if r["key"].startswith(f"hall_s{k}_w"):
                ring = rings[r["ref"]]
                P = round(sum(math.dist(p, q) for p, q in zip(ring, ring[1:] + ring[:1])))
                a, b = round(r["span"][0] * P), round(r["span"][1] * P)
                n = r["count"]
                at = ([a + (2 * i + 1) * (b - a) // (2 * n) for i in range(n)] if r["ends"] == "half"
                      else [a + i * (b - a) // (n - 1) for i in range(n)])
                got[r["key"][len(f"hall_s{k}"):]] = [point_at(ring, s, r["offset"] * 1000) for s in at]
        return got

    def assert_over(self, out, k, slack=2):
        ground, above = self.bays(out, 0), self.bays(out, k)
        self.assertEqual(sorted(ground), sorted(above))
        for key in ground:
            for p, q in zip(ground[key], above[key]):
                self.assertLessEqual(math.dist(p, q), slack, (key, p, q))

    def test_a_turned_building_keeps_its_walls_over_its_walls(self):
        """A rectangle turned off the axes has corners rounded to the millimetre, square only to a
        part in ten thousand. It still gets its footing, is never refused for a stairwell on the
        line of its own wall, and every bay above stands over the bay below (the review of K4:
        bays up to 1.5 m off, with nothing said)."""
        stair = {"side": 2, "at": 1.3, "width": 1.3, "run": 6.5}
        for degrees in (11, 30, 45, 0.5, 89.5, 123.4):
            for side in (0, 1, 2, 3):
                ring = self.turned(self.RECT, degrees)
                if side % 2:
                    stair_ = dict(stair, side=side)       # 9.6 m sides
                else:
                    stair_ = dict(stair, side=side)
                out = self.plan(ring, storeys=3, stair=stair_)
                self.assertEqual(len(out["footings"]), 2, (degrees, side))
                self.assertEqual(out["open"], [], (degrees, side))
                for k in (1, 2):
                    self.assert_over(out, k)
                for f in out["flights"]:
                    self.assertTrue(L["inside"](out["storeys"][int(f["ends_at"][6])]["region"], f["path"][-1]))

    def test_two_sides_in_one_line_are_not_mistaken(self):
        """A U: its two prongs' ends lie in one line, and the stair climbs the back. Each prong's
        wall above stands over its own wall below (the review of K4: one took the other's span)."""
        u = [(0, 0), (15600, 0), (15600, 9600), (10500, 9600), (10500, 5100), (5100, 5100), (5100, 9600), (0, 9600)]
        rows = self.ROWS + [{"id": "corner_in", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000],
                             "params": {"reach": 1.2}}]
        out = L["plan"]("hall", u, {"storeys": 2, "storey_h": 3.0, "doors": [],
                                    "stair": {"side": 0, "at": 2.8, "width": 1, "run": 5}},
                        10.0, GRID, None, kits=self.kits(rows))
        self.assertEqual(out["open"], [])
        self.assert_over(out, 1, slack=0)

    def test_a_stairwell_edge_in_line_with_a_side_is_not_mistaken(self):
        ell = [(0, 0), (9600, 0), (9600, 5100), (5100, 5100), (5100, 9600), (0, 9600)]
        rows = self.ROWS + [{"id": "corner_in", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000],
                             "params": {"reach": 1.2}}]
        # the first stairwell starts 5.1 m along side 0, in line with side 3 (x = 5100)
        for stair in ({"side": 0, "at": 6.1, "width": 1, "run": 2.4}, {"side": 5, "at": 1.3, "width": 1.3, "run": 3.0}):
            out = L["plan"]("hall", ell, {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": stair},
                            10.0, GRID, None, kits=self.kits(rows))
            self.assertEqual(out["open"], [], stair)
            self.assert_over(out, 1, slack=0)

    def test_a_wide_corner_keeps_the_stairwell_off_it(self):
        """A corner pier 0.75 m square stands with its middle 0.375 m in along both walls: past
        the footing. The stairwell is held off the corner by what the pier takes, so the pier has
        floor under it (the review of K4: it stood over the stairwell, an error in the build). A
        corner that takes less than the wall is thick does not move the bays beside it."""
        for reach, at in ((0.75, 1.3), (0.6, 1.3), (0.15, 1.0)):
            rows = [dict(r, params={"reach": reach}, bounds=[-375, -375, 0, 375, 375, 3000]) if r["id"] == "post" else r
                    for r in self.ROWS]
            ring = [(0, 0), (12600 + 2 * round(reach * 1000) - 600, 0), (12600 + 2 * round(reach * 1000) - 600, 9000 + 2 * round(reach * 1000)),
                    (0, 9000 + 2 * round(reach * 1000))]
            for side in (0, 2, 3):
                out = L["plan"]("hall", ring, {"storeys": 2, "storey_h": 3.0, "doors": [],
                                               "stair": {"side": side, "at": at, "width": 1.3, "run": 5}},
                                10.0, GRID, None, kits=self.kits(rows))
                self.assertEqual(out["open"], [], (reach, side))
                self.assert_over(out, 1, slack=0)
                rings = {"hall_s1.floor": out["storeys"][1]["region"], "hall_s1_footing.top": out["footings"][0]["region"]}
                for i in out["instances"]:
                    if i["key"].startswith("hall_s1_c"):
                        self.assertTrue(L["inside"](rings[i["surface"]], i["at"]), (reach, side, i))

    def test_a_stairwell_that_nears_another_wall_is_refused(self):
        """The stairwell keeps a wall's thickness off every side but its own: one that would reach
        into a yard's wall, or out of a narrow wing, is refused, along its whole length and not
        only at its corners. One that lies along the line of its own wall's inner face is not."""
        u = [(0, 0), (15600, 0), (15600, 9600), (10500, 9600), (10500, 1300), (5100, 1300), (5100, 9600), (0, 9600)]
        rows = self.ROWS + [{"id": "corner_in", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000],
                             "params": {"reach": 1.2}}]
        with self.assertRaises(ParamError) as c:
            L["plan"]("hall", u, {"storeys": 2, "storey_h": 3.0, "doors": [],
                                  "stair": {"side": 0, "at": 2.8, "width": 1, "run": 9}},
                      10.0, GRID, None, kits=self.kits(rows))
        self.assertIn("deeper than the building", c.exception.message)
        wing = [(0, 0), (9600, 0), (9600, 2000), (0, 2000)]
        with self.assertRaises(ParamError):
            self.plan(wing, storeys=3, stair={"side": 0, "at": 1.3, "width": 1, "run": 5})
        self.plan(wing, storeys=2, stair={"side": 0, "at": 1.3, "width": 1, "run": 5})       # one lane fits

    def test_a_variant_is_asked_of_the_storeys_it_is_asked_on(self):
        """A fill on the storey above, where the 3 m window bay stands, is no fault of the 4 m
        ground storey that has no such variant (the review of K4: a false `variant_none`)."""
        out = self.plan(ground_h=4.0, fill=[{"storeys": [1], "pattern": ["window"]}],
                        bays=[{"storey": 1, "side": 0, "at": 5, "variant": "window"}])
        self.assertEqual([n for n in out["notes"] if n["code"] == "variant_none"], [])
        self.assertEqual({r["asset"] for r in out["runs"] if r["key"].startswith("hall_s1_")}, {"window"})
        low = self.plan(ground_h=4.0, fill=[{"storeys": [0], "pattern": ["window"]}])
        self.assertEqual([(n["variant"], n["refused"]["id"]) for n in low["notes"] if n["code"] == "variant_none"],
                         [("window", "window")])

    def test_an_inner_wall_above_starts_at_the_wall_as_below(self):
        """Rooms of a storey with a footing are cut from its floor, but the wall between them is
        laid from the outline's faces, where it was before the footing: from the stairwell's edge
        (1.5 m in, at the cut) to the far wall, its opening at the middle of that, 24.75 m."""
        rooms = [{"storey": 0, "shares": [1, 1]}, {"storey": 1, "shares": [1, 1]}]
        out = L["plan"]("hall", HALL, dict(ATTRS, rooms=rooms, stair={"t": 0.025, "width": 1, "run": 5}), 10.0, GRID, KIT)
        self.assertEqual([(o["storey"], tuple(o["at"]), o["width"]) for o in out["openings"]],
                         [(0, (26000, 24000), 900), (1, (26000, 24750), 900)])
        inner = sorted(tuple(i["at"]) for i in out["instances"] if i["key"].startswith("hall_s1_i"))
        self.assertEqual(inner[0], (26000, 22300))
        self.assertAlmostEqual(sum(area(r["region"]) for r in out["rooms"] if r["storey"] == "hall_s1"),
                               area(out["storeys"][1]["region"]))

    def test_only_a_top_height(self):
        out = L["plan"]("hall", HALL, dict(ATTRS, storeys=3, top_h=4.0, roof="garden"), 10.0, GRID)
        self.assertEqual(([s["level"] for s in out["storeys"]], out["roof"]["eaves"]), ([10.3, 13.3, 16.3], 20.3))

    def test_no_wall_above_no_footing(self):
        """A kit with a bay for the ground storey only walls no storey above, and cuts no footing
        for a wall that is not there."""
        rows = [r for r in self.ROWS if r["id"].startswith("g_")]
        out = L["plan"]("hall", self.RECT, {"storeys": 2, "storey_h": 3.0, "ground_h": 4.0, "doors": [], "stair": self.STAIR},
                        10.0, GRID, None, kits=self.kits(rows))
        self.assertEqual(out["footings"], [])
        self.assertEqual({r["key"][:7] for r in out["runs"]}, {"hall_s0"})

    def test_what_height_keeps_and_leaves_out(self):
        """Pieces within the fit of the nearest height all stay; an inside corner is chosen by
        height as a post is."""
        rows = self.ROWS + [self.row("wall_b", "wall", (3000, 300), 3003, pitch=3),
                            {"id": "in3", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000], "params": {"reach": 1.2}},
                            {"id": "in4", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 4000], "params": {"reach": 1.2}}]
        ids = {r["id"] for r in L["kit_rows"](rows, "upper", 3000)}
        self.assertTrue({"wall", "wall_b", "in3"} <= ids and "in4" not in ids, ids)
        self.assertEqual(L["kit_pieces"](rows, "ground", 4000)["corner_in"]["id"], "in4")

    def test_where_a_footing_cannot_be_and_where_its_stairwell_stops(self):
        """A neighbour no longer than the wall is thick gives no footing. A stairwell that ran to
        the end corner stops a wall's thickness short of it. One that would come within a wall's
        thickness of a yard's wall is refused, though it stays inside the outline; 0.4 m off, it is
        not."""
        stub = [(0, 0), (12600, 0), (12600, 9600), (300, 9600), (300, 9300), (0, 9300)]
        self.assertFalse(L["square_ends"](stub, 2, 300))
        self.assertTrue(L["square_ends"](self.RECT, 2, 300))
        end = self.plan(stair={"side": 2, "at": 7.6, "width": 1, "run": 5})["storeys"][1]["region"]
        self.assertTrue({(300, 9300), (300, 8100)} <= set(end), end)
        rows = self.ROWS + [{"id": "corner_in", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000],
                             "params": {"reach": 1.2}}]

        def u(yard):
            ring = [(0, 0), (15600, 0), (15600, 9600), (10500, 9600), (10500, yard), (5100, yard), (5100, 9600), (0, 9600)]
            return L["plan"]("hall", ring, {"storeys": 2, "storey_h": 3.0, "doors": [],
                                            "stair": {"side": 0, "at": 2.8, "width": 1, "run": 9}},
                             10.0, GRID, None, kits=self.kits(rows))
        with self.assertRaises(ParamError):
            u(1700)                                       # 0.2 m from the yard's wall
        self.assertEqual(len(u(1900)["footings"]), 1)     # 0.4 m

    def test_no_kit_no_footing(self):
        out = L["plan"]("hall", self.RECT, {"storeys": 2, "storey_h": 3.0, "doors": [], "stair": self.STAIR}, 10.0, GRID)
        self.assertEqual((out["footings"], out["runs"]), ([], []))
        self.assertTrue(same_ring(out["storeys"][1]["region"], [(0, 0), (12600, 0), (12600, 9600), (10800, 9600),
                                                                (10800, 8400), (4800, 8400), (4800, 9600), (0, 9600)]))

    def test_a_bay_is_chosen_on_the_stair_side_above(self):
        out = self.plan(bays=[{"storey": 1, "side": 2, "at": 5.0, "variant": "window"}],
                        fill=[{"storeys": [1], "sides": [1], "pattern": ["window"]}])
        won = sorted((r["key"], r["ref"], r.get("assets") or r["asset"]) for r in out["runs"]
                     if "window" in (r.get("assets") or [r["asset"]]))
        self.assertEqual(won, [("hall_s1_w1_0", "hall_s1", "window"),
                               ("hall_s1_w2_0", "hall_s1_footing", ["wall", "window", "wall", "wall"])])
        self.assertEqual([n["code"] for n in out["notes"]], [])

    def test_rooms_of_a_storey_over_a_stairwell_fill_it(self):
        out = self.plan(rooms=[{"storey": 1, "shares": [1, 1]}])
        self.assertAlmostEqual(sum(area(r["region"]) for r in out["rooms"]), area(out["storeys"][1]["region"]))


class Decks(unittest.TestCase):
    """K4: floor tiles and roofs from pieces (rules 9 and 10)."""
    ROWS = Upward.ROWS + [
        {"id": "floor", "tags": ["role:floor"], "bounds": [-750, -750, 0, 750, 750, 200], "params": {"pitch": 1.5}},
        {"id": "slope", "tags": ["role:roof_slope"], "bounds": [-750, -2400, 0, 750, 2400, 2400],
         "params": {"pitch": 1.5, "span": 4.8}},
        {"id": "gable", "tags": ["role:roof_gable"], "bounds": [-2400, -150, 0, 2400, 150, 2400], "params": {"span": 4.8}}]
    RECT = Upward.RECT

    def plan(self, ring=None, rows=None, **attrs):
        rows = self.ROWS if rows is None else rows
        a = {"storeys": 1, "storey_h": 3.0, "doors": []}
        a.update(attrs)
        return L["plan"]("hall", ring or self.RECT, a, 10.0, GRID, None,
                         kits=lambda w, h: L["kit_pieces"](rows, w, h))

    @staticmethod
    def tiles(out, prefix):
        return [i for i in out["instances"] if i["key"].startswith(prefix + "_f") and i["asset"] == "floor"]

    def test_inset(self):
        self.assertEqual(L["inset"](self.RECT, 300), [(300, 300), (12300, 300), (12300, 9300), (300, 9300)])
        ell = [(0, 0), (9600, 0), (9600, 5100), (5100, 5100), (5100, 9600), (0, 9600)]
        self.assertEqual(L["inset"](ell, 300), [(300, 300), (9300, 300), (9300, 4800), (4800, 4800), (4800, 9300), (300, 9300)])

    def test_a_floor_is_whole_tiles_inside_the_walls(self):
        """12.6 by 9.6 m less its walls is 12 by 9 m: 8 by 6 tiles of 1.5 m, each on the floor, none
        twice, their middles on the grid from the inner corner."""
        out = self.plan()
        tiles = self.tiles(out, "hall_s0")
        self.assertEqual(len(tiles), 48)
        self.assertEqual(len({t["key"] for t in tiles}), 48)
        self.assertEqual({tuple(t["at"]) for t in tiles},
                         {(300 + 750 + 1500 * i, 300 + 750 + 1500 * j) for i in range(8) for j in range(6)})
        self.assertEqual({t["surface"] for t in tiles}, {"hall_s0.floor"})
        self.assertEqual([n for n in out["notes"] if n["code"] == "slab_open"], [])
        self.assertEqual(self.tiles(self.plan(rows=Upward.ROWS), "hall_s0"), [])      # no tile in the kit: no floor laid

    def test_an_ell_is_tiled_round_its_inside_corner(self):
        ell = [(0, 0), (9600, 0), (9600, 5100), (5100, 5100), (5100, 9600), (0, 9600)]
        rows = self.ROWS + [{"id": "corner_in", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000],
                             "params": {"reach": 1.2}}]
        out = self.plan(ell, rows)
        tiles = self.tiles(out, "hall_s0")
        self.assertEqual(len(tiles), 6 * 3 + 3 * 3)             # 9 by 4.5 m and 4.5 by 4.5 m inside the walls
        self.assertEqual([n for n in out["notes"] if n["code"] == "slab_open"], [])
        self.assertTrue(all(not (t["at"][0] > 4800 and t["at"][1] > 4800) for t in tiles))

    def test_a_stairwell_of_whole_tiles_and_one_that_is_not(self):
        """A stair 1.3 m wide makes its stairwell 1.5 m deep inside the wall, one tile: the floor
        above is whole tiles round it. At 1 m wide it is 1.2 m deep: the tiles beside it are left
        out and the note says the width that closes."""
        fits = self.plan(storeys=2, stair={"side": 2, "at": 2.8, "width": 1.3, "run": 5})
        self.assertEqual([n for n in fits["notes"] if n["code"] == "slab_open"], [])
        self.assertEqual(len(self.tiles(fits, "hall_s1")), 48 - 4)           # 6 m by 1.5 m of stairwell
        off = self.plan(storeys=2, stair={"side": 2, "at": 2.8, "width": 1, "run": 5})
        (n,) = [n for n in off["notes"] if n["code"] == "slab_open"]
        self.assertEqual((n["storey"], n["cells"], n["side"], n["deep"], n["width"], n["tiles"]), (1, 4, 2, 1200, 1300, 1))
        self.assertEqual(len(self.tiles(off, "hall_s1")), 48 - 4)
        ring = off["storeys"][1]["region"]
        for t in self.tiles(off, "hall_s1"):                                  # every tile laid lies wholly on the floor
            x, y = t["at"]
            self.assertTrue(all(L["inside"](ring, (x + dx, y + dy)) for dx in (-745, 745) for dy in (-745, 745)), t)
        long = self.plan([(0, 0), (13100, 0), (13100, 9600), (0, 9600)])       # 12.5 m inside: a third of a tile over
        (n,) = [n for n in long["notes"] if n["code"] == "slab_open"]
        self.assertEqual((n["storey"], n["cells"], "side" in n), (0, 6, False))
        both = self.plan([(0, 0), (13100, 0), (13100, 9600), (0, 9600)], storeys=2,
                         stair={"side": 2, "at": 2.8, "width": 1.3, "run": 5})
        self.assertEqual([(n["storey"], "side" in n) for n in both["notes"] if n["code"] == "slab_open"],
                         [(0, False), (1, False)])            # the plan is off the tiles, not the stairwell: it is not blamed

    def test_a_tile_is_named_by_its_cell_and_faces_side_0(self):
        out = self.plan()
        at = {t["key"]: (tuple(t["at"]), t["face"]) for t in self.tiles(out, "hall_s0")}
        self.assertEqual(at["hall_s0_f0_0"], ((1050, 1050), 180.0))
        self.assertEqual(at["hall_s0_f7_5"], ((11550, 8550), 180.0))
        self.assertEqual(at["hall_s0_f3_1"], ((5550, 2550), 180.0))

    def test_a_tile_is_laid_only_where_the_whole_of_it_has_floor(self):
        """A floor with a bite 0.6 m wide between a cell's corners: its four corners have floor,
        its middle has not, and no tile is laid there. The note stands at the first such cell.
        On a building turned off the axes no tile is laid at all, and `short` is None."""
        tile = L["kit_pieces"](self.ROWS)["floor"]
        floor = [(0, 0), (900, 0), (900, 1500), (1500, 1500), (1500, 0), (12600, 0), (12600, 9600), (0, 9600)]
        insts, (short, first) = L["slab_plan"](floor, self.RECT, 300, tile, "b", "b.floor")
        self.assertEqual((len(insts), short, tuple(first)), (47, 1, (1050, 1050)))
        self.assertNotIn("b_f0_0", {i["key"] for i in insts})
        two = [(0, 0), (900, 0), (900, 1500), (1500, 1500), (1500, 0), (12600, 0), (12600, 9000), (0, 9000)]
        _, (short, first) = L["slab_plan"](two, self.RECT, 300, tile, "b", "b.floor")
        self.assertEqual((short, tuple(first)), (9, (1050, 1050)))            # the first, not the last, of the nine
        turned = Upward.turned(self.RECT, 30)
        self.assertEqual(L["slab_plan"](turned, turned, 300, tile, "b", "b.floor"), ([], (None, None)))
        out = self.plan(turned, roof="flat")
        self.assertEqual([(n["code"], n.get("turned"), n.get("flat")) for n in out["notes"]],
                         [("slab_open", True, None), ("roof_none", None, False)])
        self.assertEqual(out["roof"], None)

    def test_a_gable_over_a_square_and_over_a_turned_rectangle(self):
        """Of equal sides the first carries the eave. A rectangle turned off the axes, its corners
        rounded to the millimetre, is still a rectangle, and its gable faces are whole
        millidegrees."""
        square_ = [(0, 0), (9600, 0), (9600, 9600), (0, 9600)]
        out = self.plan(square_, roof="gable")
        self.assertEqual(sorted(r["key"] for r in out["runs"] if r["asset"] == "slope"), ["hall_roof_e0", "hall_roof_e2"])
        for degrees in (11, 30, 77.7):
            out = self.plan(Upward.turned(self.RECT, degrees), roof="gable")
            self.assertEqual([n for n in out["notes"] if n["code"] == "roof_none"], [], degrees)
            self.assertEqual(sum(r["count"] for r in out["runs"] if r["asset"] == "slope"), 16)
            for i in out["instances"]:
                self.assertAlmostEqual(i["face"] * 1000, round(i["face"] * 1000), places=6)

    def test_a_flat_roof_is_the_floor_again_at_the_eaves(self):
        out = self.plan(roof="flat", storeys=2, stair={"side": 2, "at": 2.8, "width": 1.3, "run": 5})
        self.assertEqual(out["roof"], {"key": "hall_roof", "region": self.RECT, "eaves": 16.3})
        tiles = self.tiles(out, "hall_roof")
        self.assertEqual((len(tiles), {t["surface"] for t in tiles}), (48, {"hall_roof.garden"}))
        bare = self.plan(roof="flat", rows=Upward.ROWS)
        self.assertEqual(bare["roof"], None)
        self.assertEqual([(n["roof"], n["flat"]) for n in bare["notes"] if n["code"] == "roof_none"], [("flat", False)])
        with self.assertRaises(ParamError):
            self.plan(roof="dome")

    def test_a_gable_roof(self):
        """Slopes along the two 12.6 m sides, eight 1.5 m pieces each between the gables, their
        middles half a span in; two gable halves on each short side, the second turned about."""
        out = self.plan(roof="gable")
        self.assertEqual(out["roof"], {"key": "hall_roof", "region": self.RECT, "eaves": 13.3})
        runs = {r["key"]: r for r in out["runs"] if r["key"].startswith("hall_roof_")}
        self.assertEqual(sorted(runs), ["hall_roof_e0", "hall_roof_e2"])
        P = 2 * (12600 + 9600)
        for key, start, face in (("hall_roof_e0", 0, 180.0), ("hall_roof_e2", 12600 + 9600, 0.0)):
            r = runs[key]
            self.assertEqual((r["asset"], r["count"], r["ref"], r["offset"], r["face"], r["ends"]),
                             ("slope", 8, "hall_roof", 2.4, face, "half"))
            self.assertEqual(stations(r, self.RECT), [start + 300 + 750 + 1500 * k for k in range(8)])
        self.assertEqual(point_at(self.RECT, stations(runs["hall_roof_e2"], self.RECT)[0], 2400), (11550, 7200))
        gables = {i["key"]: (tuple(i["at"]), i["face"], i["surface"]) for i in out["instances"] if i["asset"] == "gable"}
        self.assertEqual(gables, {"hall_roof_g1_0": ((12450, 2400), 90.0, "hall_roof.garden"),
                                  "hall_roof_g1_1": ((12450, 7200), 270.0, "hall_roof.garden"),
                                  "hall_roof_g3_0": ((150, 7200), 270.0, "hall_roof.garden"),
                                  "hall_roof_g3_1": ((150, 2400), 90.0, "hall_roof.garden")})
        self.assertEqual([n for n in out["notes"] if n["code"] == "roof_none"], [])
        self.assertEqual(self.tiles(out, "hall_roof"), [])

    def test_a_gable_that_cannot_be_laid_says_why_and_falls_back(self):
        ell = [(0, 0), (9600, 0), (9600, 5100), (5100, 5100), (5100, 9600), (0, 9600)]
        no_gable = [r for r in self.ROWS if r["id"] != "gable"]
        short = [dict(r, params={"span": 4.5}) if r["id"] == "gable" else r for r in self.ROWS]
        zed = [(0, 0), (9600, 0), (9600, 5100), (14100, 5100), (14100, 9600), (5100, 9600), (5100, 5100), (0, 5100)]
        for ring, rows, word in ((ell, None, "has no `role:roof_hip` and `role:roof_valley`"),
                                 (zed, None, "neither a rectangle nor an L"),
                                 ([(0, 0), (12600, 0), (12600, 11100), (0, 11100)], None, "11.1 m deep"),
                                 ([(0, 0), (13100, 0), (13100, 9600), (0, 9600)], None, "12.5 m, no whole number"),
                                 (None, no_gable, "has no `role:roof_slope` and `role:roof_gable`"),
                                 (None, short, "span of 4.5 m")):
            out = self.plan(ring, rows, roof="gable")
            (n,) = [n for n in out["notes"] if n["code"] == "roof_none"]
            self.assertIn(word, n["why"])
            self.assertTrue(n["flat"])
            self.assertTrue(self.tiles(out, "hall_roof"), word)
            self.assertEqual([r for r in out["runs"] if r["asset"] == "slope"], [])
            self.assertEqual(out["roof"]["key"], "hall_roof")
        self.assertIn("14.1 or 15.6 m", self.plan([(0, 0), (14600, 0), (14600, 9600), (0, 9600)], roof="gable")["notes"][-1]["why"])
        none = L["plan"]("hall", self.RECT, {"storeys": 1, "storey_h": 3.0, "doors": [], "roof": "gable"}, 10.0, GRID)
        self.assertEqual([(n["code"], n["flat"]) for n in none["notes"]], [("roof_none", False)])

    def test_the_ridge_lies_along_the_longer_sides(self):
        tall = [(0, 0), (9600, 0), (9600, 12600), (0, 12600)]
        out = self.plan(tall, roof="gable")
        self.assertEqual(sorted(r["key"] for r in out["runs"] if r["asset"] == "slope"), ["hall_roof_e1", "hall_roof_e3"])
        self.assertEqual(sorted(i["key"] for i in out["instances"] if i["asset"] == "gable"),
                         ["hall_roof_g0_0", "hall_roof_g0_1", "hall_roof_g2_0", "hall_roof_g2_1"])


class Stated(unittest.TestCase):
    """Rule 11: a kit states once what its pieces share, and `lay` lays by it."""

    @staticmethod
    def rows(shared=None, thick=300):
        def row(id_, role, size, **params):
            w, d = size
            return {"id": id_, "tags": [f"role:{role}"], "bounds": [-w // 2, -d // 2, 0, w // 2, d // 2, 3000],
                    "params": params, "shared": shared or {}}
        return [row("wall", "wall", (3000, thick), pitch=3), row("half", "filler", (1500, thick), pitch=1.5),
                row("corner", "corner", (300, 300)), row("door", "door", (1500, thick), pitch=1.5),
                row("tile", "floor", (1500, 1500), pitch=1.5)]

    PLAIN = {"module": 3.0, "wall": 0.3, "storey": 3.0, "door": 1.4,
             "stair": {"lane": 1.5, "width": 1.3, "run": 4.5, "landing": 1.5}}
    HALL = [(10000, 10000), (22600, 10000), (22600, 19600), (10000, 19600)]

    def test_what_a_kit_states_is_read_in_millimetres(self):
        got = L["kit_shared"](self.rows(dict(self.PLAIN, storey=[4.0, 3.0])))
        self.assertEqual(got, {"module": 3000, "wall": 300, "storey": [3000, 4000], "storey_first": 4000,
                               "door": 1400, "stair": {"lane": 1500, "width": 1300, "run": 4500, "landing": 1500}})
        self.assertEqual(L["kit_shared"](self.rows()), {})
        self.assertEqual(L["kit_shared"](None), {})
        # A key `lay` does not read is another library's: `shared` holds any keys.
        self.assertEqual(L["kit_shared"](self.rows({"wall": 0.3, "street": {"kerb": 0.15}})), {"wall": 300})

    def test_a_pitch_and_a_span_the_pack_leaves_out_are_the_models(self):
        """The fix plan's F3b: a piece's pitch is its box's length; between posts a panel's and a
        door's pitch is its box and the post; a slope's and a hip's span is its depth, a gable
        half's its length. A stated number wins."""
        def row(i, role, b, **params):
            return {"id": i, "tags": [f"role:{role}"], "bounds": b, "params": params, "shared": {}}
        rows = [row("bay", "wall", [-1200, -150, 0, 1200, 150, 3000]), row("post", "post", [-150, -150, 0, 150, 150, 3000]),
                row("door", "door", [-1200, -150, 0, 1200, 150, 3000]),
                row("slope", "roof_slope", [-1350, -3000, 0, 1350, 3000, 1500]),
                row("gable", "roof_gable", [-2850, -150, 0, 2850, 150, 1500]),
                row("hip", "roof_hip", [-3000, -3000, 0, 3000, 3000, 1500])]
        k = L["kit_pieces"](rows, "ground", 3000)
        self.assertEqual((k["wall"]["pitch"], k["door"]["pitch"]), (2700, 2700))      # 2.4 m and a 0.3 m post
        self.assertEqual(k["roofs"]["roof_slope"][0]["span"], 6000)
        self.assertEqual(k["roofs"]["roof_gable"][0]["span"], 5700)
        self.assertEqual(k["roofs"]["roof_hip"][0]["span"], 6000)
        rows[0] = row("bay", "wall", [-1200, -150, 0, 1200, 150, 3000], pitch=2.6)       # stated: kept
        rows[3] = row("slope", "roof_slope", [-1350, -3000, 0, 1350, 3000, 1500], span=5.5)
        k = L["kit_pieces"](rows, "ground", 3000)
        self.assertEqual((k["wall"]["pitch"], k["roofs"]["roof_slope"][0]["span"]), (2600, 5500))
        # with no post a panel's pitch is its box alone
        k = L["kit_pieces"]([r for r in rows if r["id"] != "post"], "ground", 3000)
        self.assertEqual(k["door"]["pitch"], 2400)
        # F3b's review: a variant is matched after the post is added, whichever of the two states it
        bay = row("bay", "wall", [-1200, -150, 0, 1200, 150, 3000], pitch=2.7)
        var = dict(row("arch", "wall", [-1200, -150, 0, 1200, 150, 3000]), tags=["role:wall", "variant:arch"])
        k = L["kit_pieces"]([bay, var, rows[1]], "ground", 3000)
        self.assertIn("arch", k["wall"]["variants"])
        self.assertEqual(k["wall"]["variants"]["arch"]["pitch"], 2700)
        # and a door of another storey's group takes no post's width
        door = dict(row("door", "door", [-1200, -150, 0, 1200, 150, 3000]))
        tagged = [dict(r, tags=r["tags"] + ["storey:colonnade"]) for r in (row("bay", "wall", [-1200, -150, 0, 1200, 150, 3000]),
                                                                          row("post", "post", [-150, -150, 0, 150, 150, 3000]))]
        k = L["kit_pieces"](tagged + [door], "colonnade", 3000)
        self.assertEqual((k["wall"]["pitch"], k["door"]["pitch"]), (2700, 2400))

    def test_a_building_takes_the_kits_storey_and_plinth_where_it_writes_none(self):
        """Rule 2, the fix plan's F3: `storey_h = 0` and `plinth = -1` (the defaults) are the kit's
        first stated storey and its stated plinth, else 3.2 and 0.3; a number written wins."""
        st = L["kit_shared"](self.rows(dict(self.PLAIN, storey=[4.0, 3.0], plinth=0)))
        self.assertEqual(st["plinth"], 0)
        a = L["kit_heights"]({"storey_h": 0, "plinth": -1}, st)
        self.assertEqual((a["storey_h"], a["plinth"]), (4.0, 0.0))
        a = L["kit_heights"]({"storey_h": 3.5, "plinth": 0.45}, st)
        self.assertEqual((a["storey_h"], a["plinth"]), (3.5, 0.45))
        a = L["kit_heights"]({}, {})
        self.assertEqual((a["storey_h"], a["plinth"]), (3.2, 0.3))
        with self.assertRaises(L["ParamError"]):
            L["kit_shared"](self.rows(dict(self.PLAIN, plinth=-0.1)))
        # `storey = []` states no storey, as before F3 (its review): the building's own, else 3.2
        self.assertNotIn("storey_first", L["kit_shared"](self.rows(dict(self.PLAIN, storey=[]))))
        self.assertEqual(C["kit_storey"](self.rows(dict(self.PLAIN, storey=[]))), 3.2)
        self.assertEqual(C["kit_storey"](self.rows(dict(self.PLAIN, storey=[4.0, 3.0]))), 4.0)
        self.assertEqual(C["kit_plinth"](self.rows(dict(self.PLAIN, plinth=0))), 0.0)
        self.assertEqual((C["kit_storey"]([]), C["kit_plinth"]([])), (3.2, 0.3))

    def test_a_kit_says_a_thing_once_and_as_a_length(self):
        two = self.rows(self.PLAIN)
        two[1] = dict(two[1], shared={"module": 4.0})
        for rows, word in ((two, "two `shared` tables"),
                           (self.rows({"wall": 0}), "not a length"),
                           (self.rows({"wall": "thin"}), "not a length"),
                           (self.rows({"wall": True}), "not a length"),
                           (self.rows({"stair": {"lanes": 1.5}}), "shared.stair"),
                           (self.rows({"stair": 1.5}), "shared.stair"),
                           (self.rows({"stair": {"lane": 1.0, "width": 1.3}}), "wide in a lane")):
            with self.assertRaises(L["ParamError"]) as c:
                L["kit_shared"](rows)
            self.assertEqual(c.exception.key, "kit")
            self.assertIn(word, c.exception.message)

    def test_the_stated_wall_is_the_thickness_laid_by(self):
        """A bay re-exported 0.35 m thick in a kit that states 0.3 m walls: the wall is laid as the
        kit states, and the bay's own measure is kept for `check`."""
        kit = L["kit_pieces"](self.rows(self.PLAIN, thick=350))
        self.assertEqual((kit["wall"]["thick"], kit["wall"]["measured"]), (300, 350))
        self.assertEqual(kit["wall"]["stated"]["wall"], 300)
        loose = L["kit_pieces"](self.rows(None, thick=350))
        self.assertEqual((loose["wall"]["thick"], loose["wall"]["measured"], loose["wall"]["stated"]), (350, 350, {}))

    def plan(self, attrs, shared=PLAIN, storeys=2):
        rows = self.rows(shared)
        a = {"storeys": storeys, "storey_h": 3.0, "plinth": 0.3, "porch": 2, "walk_out": 3,
             "doors": [{"storey": 0, "side": 0, "at": 4.05}], **attrs}
        return L["plan"]("hall", self.HALL, a, 10.0, GRID, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))

    def test_a_stair_takes_the_kits_width_run_and_lane(self):
        """The building writes where the stair is; its width, run, landing and lane are the kit's.
        Lane 0's middle is half a lane inside the wall's inner face, and the stairwell is one
        lane deep and a landing and a run long: 1.5 m by 6 m, one tile by four, from 1.5 m along
        the wall's inner face."""
        out = self.plan({"stair": {"side": 2, "at": 3.3}})
        flight = out["flights"][0]
        self.assertEqual(flight["width"], 1.3)
        (x0, y0), (x1, y1) = flight["path"]
        self.assertEqual((abs(x1 - x0), y0, y1), (4500, 19600 - 300 - 750, 19600 - 300 - 750))
        self.assertFalse([n for n in out["notes"] if n["code"] == "slab_open"])

    def test_three_storeys_of_a_kit_with_a_lane_of_tiles_leave_no_floor_open(self):
        """The second stairwell is two lanes deep and lies a landing further along. With the
        library's own lanes (the width and 0.1 m each side) and its 1 m landing it is 2.9 m deep
        and starts 2.5 m along: no whole number of 1.5 m tiles either way. With the kit's lane
        and landing of one tile it is 3 m deep from 3 m along."""
        stated = self.plan({"stair": {"side": 2, "at": 3.3}}, storeys=3)
        self.assertFalse([n for n in stated["notes"] if n["code"] == "slab_open"])
        loose = self.plan({"stair": {"side": 2, "at": 2.8, "width": 1.3, "run": 5}},
                          shared={"wall": 0.3}, storeys=3)
        self.assertTrue([n for n in loose["notes"] if n["code"] == "slab_open"])

    def test_a_stairwell_between_tiles_says_how_far_to_move_the_stair(self):
        """The kit's lane, landing and run are whole tiles, so a stairwell off the grid is off by
        where the stair stands: half a metre early here, and the note holds the two moves."""
        out = self.plan({"stair": {"side": 2, "at": 2.8}})
        note = next(n for n in out["notes"] if n["code"] == "slab_open")
        self.assertEqual((note["deep_whole"], note["long_whole"], note["from"], note["to"]), (True, True, 1000, 7000))
        self.assertEqual(note["moves"], [-1000, 500])

    def test_a_building_may_still_write_its_own_stair(self):
        out = self.plan({"stair": {"side": 2, "at": 2.8, "width": 1.0, "run": 4.5}})
        self.assertEqual(out["flights"][0]["width"], 1.0)
        (x0, _), (x1, _) = out["flights"][0]["path"]
        self.assertEqual(abs(x1 - x0), 4500)
        with self.assertRaises(L["ParamError"]) as c:
            self.plan({"stair": {"side": 2, "at": 2.8, "width": 1.6}})
        self.assertIn("the kit's lane is 1.5 m", c.exception.message)

    def test_a_stair_with_no_run_anywhere_is_refused(self):
        with self.assertRaises(L["ParamError"]) as c:
            self.plan({"stair": {"side": 2, "at": 3.3}}, shared={"wall": 0.3})
        self.assertEqual(c.exception.key, "stair")
        self.assertIn("no `run`", c.exception.message)

    def test_a_door_takes_the_kits_width(self):
        """The plain kit's door piece is made for a 1.4 m doorway: a door that writes no width gets
        it, and the south wall closes round it as it does round a door that writes 1.4."""
        stated = self.plan({"stair": {"side": 2, "at": 3.3}})
        written = self.plan({"stair": {"side": 2, "at": 3.3}, "doors": [{"storey": 0, "side": 0, "at": 4.05, "width": 1.4}]})
        self.assertEqual(stated["runs"], written["runs"])
        self.assertFalse(stated["open"])
        self.assertEqual(L["door_plan"](self.HALL, {"side": 0, "at": 4.05}, 2000, 3000)["width"], 1000)


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
    def test_a_kit_with_no_piece_is_said_and_nothing_laid(self):
        """F3's review: a misspelt kit laid a building with no walls and said nothing, and would
        take the default heights. It is `params` now."""
        class Empty(FakeStep):
            def pack(self, select, columns=()):
                return {"id": []}
        s = Empty(dict(ATTRS, kit="building/kit/nope"))
        L["generate"](s)
        self.assertEqual([x[:2] for x in s.said], [("error", "params")])
        self.assertEqual(s.said[0][3], {"attribute": "kit"})
        self.assertEqual(s.out.features, {})

    def test_generate_writes_the_house(self):
        s = FakeStep(dict(ATTRS, kit="building/kit/box", rooms=[{"storey": 0, "shares": [2, 1]}]))
        L["generate"](s)
        # The floor stands its plinth above level ground and this kit has no base: said, as an info.
        self.assertEqual([x[:2] for x in s.said], [("info", "plinth_bare"), ("info", "laid")])
        f = s.out.features
        self.assertEqual(sorted(f), ["hall_d0", "hall_f0", "hall_p0", "hall_s0", "hall_s0_r0", "hall_s0_r1",
                                     "hall_s1", "hall_s1_footing"])
        self.assertEqual((f["hall_s1_footing"]["kind"], f["hall_s1_footing"]["level"]), ("footing", 13.3))
        self.assertEqual(f["hall_s1"]["level"], 10.3 + 3.0)
        self.assertEqual(f["hall_d0"]["ends_at"], "hall_s0.floor")
        self.assertEqual(f["hall_s0_r0"]["storey"], "hall_s0")
        runs = [k for (sec, k) in s.out.tables if sec == "run"]
        self.assertTrue(runs)
        on = s.out.tables[("run", runs[0])]["on"]
        self.assertIn(on["ref"], ("hall_s0", "hall_s1", "hall_s1_footing"))
        self.assertEqual(sorted({i["surface"] for i in s.out.instances.values()}),
                         ["hall_s0.floor", "hall_s1.floor", "hall_s1_footing.top"])

    def test_variants_are_said(self):
        """`generate` says a word the kit has not, a variant it refuses (naming the piece), and a
        `bays` point no bay holds; and lays the variant it has."""
        class Kit(FakeStep):
            def pack(self, select, columns=()):
                rows = super().pack(select, columns)
                rows["id"] += ["bay_glass", "bay_wide"]
                rows["tags"] += ['["role:wall", "variant:glass"]', '["role:wall", "variant:wide"]']
                rows["bounds"] += [[-500, -150, 0, 500, 150, 3000], [-600, -150, 0, 600, 150, 3000]]
                rows["params"] += ['{"pitch": 1}', '{"pitch": 1}']
                return rows
        s = Kit(dict(ATTRS, kit="building/kit/box", fill=[{"storeys": [0], "sides": [1], "pattern": ["glass", "wide", "bow"]}],
                     bays=[{"storey": 0, "side": 1, "at": 0.1, "variant": "glass"}]))
        L["generate"](s)
        said = [(x[0], x[1], (x[3] or {}).get("variant"), (x[3] or {}).get("asset")) for x in s.said if x[0] != "info"]
        self.assertEqual(sorted(said), [("warning", "bay_none", "glass", None), ("warning", "variant_none", "bow", ""),
                                        ("warning", "variant_none", "wide", "bay_wide")])
        glass = [k for (sec, k), v in s.out.tables.items() if sec == "run" and "bay_glass" in v.get("assets", [])]
        self.assertTrue(glass and all(k.startswith("hall_s0_w1_") for k in glass), glass)
        self.assertTrue(all("asset" not in s.out.tables[("run", k)] for k in glass))

    def test_floors_and_roofs_are_said_and_written(self):
        class Kit(FakeStep):
            def pack(self, select, columns=()):
                rows = super().pack(select, columns)
                rows["id"] += ["tile"]
                rows["tags"] += ['["role:floor"]']
                rows["bounds"] += [[-500, -500, 0, 500, 500, 200]]
                rows["params"] += ['{"pitch": 1}']
                return rows
        s = Kit(dict(ATTRS, kit="building/kit/box", roof="gable"))
        L["generate"](s)
        said = sorted((x[1], (x[3] or {}).get("storey"), (x[3] or {}).get("fallback")) for x in s.said if x[0] == "warning")
        self.assertEqual(said, [("roof_none", None, "flat"), ("slab_open", -1, None), ("slab_open", 0, None),
                                ("slab_open", 1, None)])
        self.assertEqual(s.out.features["hall_roof"]["kind"], "roof")
        self.assertTrue(any(k.startswith("hall_roof_f") for k in s.out.instances))

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
