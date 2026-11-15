"""Tests of lib/review/form.py on hand-made lines (no engine). Run: python -m unittest lib/review/test_form.py"""

import os
import runpy
import unittest

form = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), "form.py"))


def degrees(paths):
    return sorted(n["degree"] for n in form["junctions"](paths) if n["degree"] != 2)


class Junctions(unittest.TestCase):
    def test_plus_is_four_way_and_four_ends(self):
        self.assertEqual(degrees([[(0, 50), (100, 50)], [(50, 0), (50, 100)]]), [1, 1, 1, 1, 4])

    def test_tee_is_three_way(self):
        # a street ending exactly on another: a T, not a crossing and not a dead end
        self.assertEqual(degrees([[(0, 0), (100, 0)], [(50, 0), (50, 60)]]), [1, 1, 1, 3])

    def test_end_within_touch_meets(self):
        self.assertEqual(degrees([[(0, 0), (100, 0)], [(50, 0.03), (50, 60)]]), [1, 1, 1, 3])

    def test_end_short_of_line_is_dead_end(self):
        self.assertEqual(degrees([[(0, 0), (100, 0)], [(50, 1), (50, 60)]]), [1, 1, 1, 1])

    def test_way_running_on_into_another_is_no_junction(self):
        self.assertEqual(degrees([[(0, 0), (50, 0)], [(50, 0), (100, 0)]]), [1, 1])

    def test_closed_ring_with_one_tee(self):
        ring = [(0, 0), (40, 0), (40, 40), (0, 40), (0, 0)]
        self.assertEqual(degrees([ring, [(40, 20), (80, 20)]]), [1, 3])

    def test_stub_piece(self):
        nodes = form["junctions"]([[(0, 0), (100, 0)], [(50, 0), (50, 30)]])
        end = [n for n in nodes if n["at"] == (50, 30)][0]
        self.assertAlmostEqual(end["piece"], 30.0)


class Bearings(unittest.TestCase):
    def test_grid_is_zero_spread_and_full_order(self):
        grid = [[(0, y), (100, y)] for y in (0, 50, 100)] + [[(x, 0), (x, 100)] for x in (0, 50, 100)]
        self.assertEqual(form["spread_90"](grid), 0.0)
        self.assertEqual(form["order"](grid), 1.0)

    def test_every_direction_alike_is_full_spread_and_no_order(self):
        import math
        star = [[(0, 0), (math.sin(math.radians(a + 2.5)), math.cos(math.radians(a + 2.5)))] for a in range(0, 360, 5)]
        self.assertGreater(form["spread_90"](star), 0.99)
        self.assertLess(form["order"](star), 0.01)


class Decks(unittest.TestCase):
    def test_one_crossing_no_wading(self):
        water = [{"name": "river", "points": [(50, 0), (50, 100)]}]
        crossing = [{"name": "bridge", "points": [(40, 50), (60, 50)]}]
        ways = [("w", "street", [(0, 50), (40, 50)]), ("e", "street", [(60, 50), (100, 50)])]
        got = {r[0]: r[2] for r in form["measure_decks"](water, crossing, ways)}
        self.assertEqual(got, {"decks": 1, "doubled_decks": 0, "ways_over_water": 0})

    def test_way_wading_and_doubled_deck(self):
        water = [{"name": "river", "points": [(50, 0), (50, 100)]}]
        crossing = [{"name": "a", "points": [(40, 50), (60, 50)]},
                    {"name": "b", "points": [(40, 55), (60, 55)]}]
        ways = [("x", "street", [(0, 90), (100, 90)])]
        got = {r[0]: r[2] for r in form["measure_decks"](water, crossing, ways)}
        self.assertEqual(got, {"decks": 2, "doubled_decks": 1, "ways_over_water": 1})


if __name__ == "__main__":
    unittest.main()
