"""Tests for the ways library's steps: parapet.py (the runs) and check.py (the crossing lint).

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/ways -p 'test_*.py'

No `via` binary is needed; test_crossing_world.py builds them in a world.
"""

import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PAR = runpy.run_path(os.path.join(HERE, "parapet.py"))
CHK = runpy.run_path(os.path.join(HERE, "check.py"))
P = CHK["params_of"]({})
BRIDGE = {"name": "bridge", "path": [(51000, 40000), (69000, 40000)], "deck_z": 21.72, "freeboard": 1.2}
MEET = [{"a": "lane_west.end", "b": "bridge.start", "gap": 0}, {"a": "lane_east.start", "b": "bridge.end", "gap": 0},
        {"a": "bridge.start", "b": "bridge_west", "gap": 0}, {"a": "bridge.end", "b": "bridge_east", "gap": 0}]


def codes(recs):
    return [(r[1], r[2]) for r in recs if r[0] != "info"]


class Parapet(unittest.TestCase):
    def test_two_runs_inside_the_bound(self):
        """A 4 m deck and a 0.15 m post: offsets +-1.925 m, the post's outer face on the edge."""
        runs = PAR["runs"]("bridge", 4, "fence_post", 150, 2.0)
        self.assertEqual([(r["key"], r["on"]["offset"]) for r in runs],
                         [("bridge_parapet_l", 1.925), ("bridge_parapet_r", -1.925)])
        self.assertTrue(all(r["on"]["span"] == [0, 1] and r["ends"] == "both" for r in runs))

    def test_span_inset_by_half_the_post(self):
        """A 0.15 m deep post on an 18 m deck: span [0.075 / 18, 1 - 0.075 / 18], so the end posts'
        faces stand on the deck's ends, not 75 mm past them."""
        runs = PAR["runs"]("bridge", 4, "fence_post", 150, 2.0, 150, 18000)
        s = round(75 / 18000, 6)
        self.assertEqual([r["on"]["span"] for r in runs], [[s, round(1 - s, 6)]] * 2)
        self.assertEqual(PAR["length_of"]([(0, 0), (3000, 4000), (3000, 10000)]), 11000)

    def test_a_piece_as_wide_as_the_deck_lays_nothing(self):
        self.assertIsNone(PAR["runs"]("bridge", 0.15, "fence_post", 150, 2.0))

    def test_params(self):
        self.assertEqual(PAR["params_of"]({})["asset"], "@prop/fence")
        for bad in ({"spacing": 0}, {"asset": ""}, {"pitch": 1}):
            with self.assertRaises(PAR["ParamError"]):
                PAR["params_of"](bad)


class Lint(unittest.TestCase):
    def test_water_beneath_and_both_ends_met(self):
        """Ground 19.2 m and water 19.5 m under the middle; the deck 2.2 m over the water."""
        recs = CHK["judge"]([BRIDGE], MEET, {"bridge": (19.2, 19.5)}, P)
        self.assertEqual(codes(recs), [])
        self.assertEqual(recs[-1][5], {"crossings": 1, "ends": 2})

    def test_dry(self):
        """No water under the middle (the stand-in deck made the ground `deck_z` there), or no
        water field at all."""
        for water in (21.72, None):
            recs = CHK["judge"]([BRIDGE], MEET, {"bridge": (21.72, water)}, P)
            self.assertEqual(codes(recs), [("dry", "bridge")])
            self.assertEqual(recs[0][6], (60.0, 40.0))

    def test_low_deck(self):
        recs = CHK["judge"]([dict(BRIDGE, freeboard=2.5)], MEET, {"bridge": (19.2, 19.5)}, P)
        self.assertEqual(codes(recs), [("low_deck", "bridge")])
        self.assertAlmostEqual(recs[0][5]["water"], 19.5)

    def test_an_end_met_only_by_its_own_point_is_unmet(self):
        """`bridge.end` meets `bridge_east` (where the crossing's end is written), but no way."""
        recs = CHK["judge"]([BRIDGE], [m for m in MEET if m["a"] != "lane_east.start"], {"bridge": (19.2, 19.5)}, P)
        self.assertEqual(codes(recs), [("deck_unmet", "bridge.end")])
        self.assertEqual(recs[0][6], (69.0, 40.0))

    def test_a_gap(self):
        meet = [dict(m, gap=300) if m["a"] == "lane_west.end" else m for m in MEET]
        recs = CHK["judge"]([BRIDGE], meet, {"bridge": (19.2, 19.5)}, P)
        self.assertEqual(codes(recs), [("deck_gap", "bridge.start")])
        self.assertEqual(recs[0][5], {"gap": 300, "way": "lane_west.end"})

    def test_middle_is_half_the_arc_length(self):
        self.assertEqual(CHK["middle"]([(0, 0), (10000, 0), (10000, 10000)]), (10000, 0))
        self.assertEqual(CHK["middle"]([(0, 0), (3000, 4000)]), (1500, 2000))


if __name__ == "__main__":
    unittest.main()
