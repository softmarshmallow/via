"""Tests for houses.py's corner houses (rule 2c) and shallow houses in wedges (rule 2b).

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/town -p 'test_*.py'

The stand-in step is test_streets.py's: two streets meet at the origin, `streets#a` along y = 0
(a main street) and `streets#b` along x = 0.
"""

import math
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
T = runpy.run_path(os.path.join(HERE, "test_streets.py"))
H = T["H"]
T["BOUNDS"].update({"house_corner": [-3000, -3000, 0, 3000, 3000, 8000],
                    "house_shallow": [-3000, -2250, 0, 3000, 2250, 6000]})

# The block's corner at the origin cut by its bisector (geometry.md 3.6) into two triangles, each
# with its sharp end at the corner: `ca` fronts the street along y = 0, `cb` the one along x = 0.
CA = [(0, 0), (12, 0), (12, 12)]
CB = [(0, 0), (12, 12), (0, 12)]


def rows(step):
    return next(s[3] for s in step.said if s[1] == "rows")


class CornerTest(unittest.TestCase):
    def test_pairs_the_two_triangles_at_a_corner(self):
        segs = [((0, 0), (100, 0)), ((0, 0), (0, 100))]
        lots = [(n, r, H["lot_frame"](r, segs)) for n, r in (("lots#ca_0", CA), ("lots#cb_0", CB))]
        pairs = H["corner_pairs"](lots)
        self.assertEqual([(a, b) for a, b, _ in pairs], [("lots#ca_0", "lots#cb_0")])
        self.assertLess(math.dist(pairs[0][2], (0, 0)), 1e-9)

    def test_no_pair_along_one_street(self):
        # Two lots side by side on one street share an edge and a chord end but do not turn.
        segs = [((0, 0), (100, 0))]
        a, b = [(20, 0), (30, 0), (30, 10), (20, 10)], [(30, 0), (40, 0), (40, 10), (30, 10)]
        lots = [(n, r, H["lot_frame"](r, segs)) for n, r in (("lots#ca_0", a), ("lots#cb_0", b))]
        self.assertEqual(H["corner_pairs"](lots), [])

    def test_cuts_of_two_lengths_still_pair(self):
        # quay: one lot's cut from the corner is 12.4 m, the other's 16.1 m, along one bisector.
        segs = [((0, 0), (100, 0)), ((0, 0), (0, 100))]
        a = [(0, 0), (11.4, 0), (11.4, 11.4)]
        b = [(0, 0), (12.4, 12.4), (0, 12.4)]
        lots = [(n, r, H["lot_frame"](r, segs)) for n, r in (("lots#ca_0", a), ("lots#cb_0", b))]
        self.assertEqual(len(H["corner_pairs"](lots)), 1)

    def test_corner_house_faces_both_streets(self):
        step = T["houses_step"]([("lots#ca_0", 2, CA, "cell"), ("lots#cb_0", 2, CB, "cell")], assets=["house_narrow"],
                                corner=["house_corner"])
        self.assertEqual(len(step.out.instances), 1)
        key, asset, at, face, _ = step.out.instances[0]
        # Its front to the street along y = 0 (south, 180), so its right side, where the side door
        # is, faces west onto the street along x = 0; both walls `setback` (1 m) from the streets.
        self.assertEqual((key, asset, face), ("ha_0", "house_corner", 180.0))
        self.assertEqual(at, (4000, 4000))
        front = (math.sin(math.radians(face)), math.cos(math.radians(face)))
        right = (front[1], -front[0])
        self.assertAlmostEqual(right[0], -1.0)        # west, onto `streets#b`
        r = rows(step)
        self.assertEqual((r["corners"], r["corner_lots"], r["lots_held"], r["empty"]), (1, 2, 2, 0))

    def test_facing_is_by_the_streets_not_the_names(self):
        # The two lots named the other way round: the house is named for the first name, and
        # still fronts the street along y = 0 with its side door to the one along x = 0.
        step = T["houses_step"]([("lots#cb_0", 2, CA, "cell"), ("lots#ca_0", 2, CB, "cell")],
                                assets=["house_narrow"], corner=["house_corner"])
        key, _, at, face, _ = step.out.instances[0]
        self.assertEqual((key, at, face), ("ha_0", (4000, 4000), 180.0))

    def test_keep_out_pushes_each_side_on_its_own(self):
        # The street along y = 0 keeps 3 m clear, the one along x = 0 only 1.5 m: the front is
        # pushed back to 3 m and the side to 1.5 m (rule 4, the least total push that is clear).
        ko = lambda x, y: 1.0 if (y < 2.99 or x < 1.49) else 0.0  # noqa: E731
        step = T["houses_step"]([("lots#ca_0", 2, CA, "cell"), ("lots#cb_0", 2, CB, "cell")], assets=["house_narrow"],
                                corner=["house_corner"], keep_out=ko)
        _, _, at, _, _ = step.out.instances[0]
        self.assertEqual(at, (4500, 6000))

    def test_a_corner_too_small_is_left_empty(self):
        small_a, small_b = [(0, 0), (6, 0), (6, 6)], [(0, 0), (6, 6), (0, 6)]
        step = T["houses_step"]([("lots#ca_0", 2, small_a, "cell"), ("lots#cb_0", 2, small_b, "cell")], assets=["house_narrow"],
                                corner=["house_corner"])
        self.assertEqual(step.out.instances, [])
        self.assertEqual(rows(step)["empty"], 2)


class WedgeTest(unittest.TestCase):
    LOT = [(20, 3), (34, 3), (34, 10), (20, 10)]       # 14 m front, 7 m deep: no 8 m house fits

    def test_shallow_house_where_no_row_fits(self):
        step = T["houses_step"]([("lots#cw_0", 2, self.LOT, "cell")], assets=["house_narrow"], fallback=["house_shallow"])
        self.assertEqual([i[1] for i in step.out.instances], ["house_shallow", "house_shallow"])
        r = rows(step)
        self.assertEqual((r["fallback_lots"], r["lots_held"], r["empty"]), (1, 1, 0))

    def test_without_fallback_it_stays_empty(self):
        step = T["houses_step"]([("lots#cw_0", 2, self.LOT, "cell")], assets=["house_narrow"])
        self.assertEqual(step.out.instances, [])
        self.assertEqual(rows(step)["empty"], 1)

    def test_fallback_slides_along_a_wedge(self):
        # Ways keep x < 25 and x > 31.5 clear: a 6 m house stands clear only between, 1 m off the
        # frontage's middle; neither centred, nor at the start or end (the old three places).
        narrow = [(20, 3), (34, 3), (34, 10), (20, 10)]
        ko = lambda x, y: 1.0 if (x < 24.99 or x > 31.51) else 0.0  # noqa: E731
        step = T["houses_step"]([("lots#cw_0", 2, narrow, "cell")], assets=["house_narrow"], fallback=["house_shallow"],
                                keep_out=ko, gap=0.25)
        self.assertEqual(len(step.out.instances), 1)
        _, asset, at, _, _ = step.out.instances[0]
        self.assertEqual(asset, "house_shallow")
        self.assertEqual(at[0], 28000)

    def test_a_row_that_stands_clear_is_kept(self):
        # A lot deep enough for the row: `fallback` is not used.
        deep = [(20, 3), (34, 3), (34, 19), (20, 19)]
        step = T["houses_step"]([("lots#cw_0", 2, deep, "cell")], assets=["house_narrow"], fallback=["house_shallow"])
        self.assertNotIn("house_shallow", [i[1] for i in step.out.instances])
        self.assertEqual(rows(step)["fallback_lots"], 0)

    def test_fallback_after_keep_out_drops_the_row(self):
        # The row fits the lot but its 8 m houses stand in the back street's keep_out (y > 14);
        # the shallow house in front of it is clear: rule 2b after rule 4.
        deep = [(20, 3), (34, 3), (34, 16), (20, 16)]
        ko = lambda x, y: 1.0 if y > 10.6 else 0.0  # noqa: E731
        step = T["houses_step"]([("lots#cw_0", 2, deep, "cell")], assets=["house_narrow"], fallback=["house_shallow"],
                                keep_out=ko)
        self.assertEqual({i[1] for i in step.out.instances}, {"house_shallow"})
        self.assertEqual(rows(step)["fallback_lots"], 1)


if __name__ == "__main__":
    unittest.main()
