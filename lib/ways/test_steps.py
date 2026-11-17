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
        self.assertEqual(PAR["refusal"](0.15, 150), "narrow")

    def test_a_deck_no_longer_than_the_piece_is_short_not_narrow(self):
        """A 4 m deck 0.15 m long and a 0.15 m deep post: the insets meet, so `short`, not `narrow`
        (which says the piece is wider than the deck: false here). 0.2 m long leaves a run."""
        self.assertIsNone(PAR["runs"]("bridge", 4, "fence_post", 150, 2.0, 150, 150))
        self.assertEqual(PAR["refusal"](4, 150, 150, 150), "short")
        self.assertIsNone(PAR["refusal"](4, 150, 150, 200))
        self.assertIsNotNone(PAR["runs"]("bridge", 4, "fence_post", 150, 2.0, 150, 200))

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
        self.assertEqual(recs[0][4], "raise `deck_z` to 22.00 or more")

    def test_a_deck_under_the_water(self):
        """A deck below the water says it stands under it (M7 close: "-0.082 m over the water")."""
        recs = CHK["judge"]([dict(BRIDGE, deck_z=19.4)], MEET, {"bridge": (19.2, 19.5)}, P)
        self.assertEqual(codes(recs), [("low_deck", "bridge")])
        self.assertIn("0.100 m under the water", recs[0][3])

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

    def test_an_end_beside_a_bridgehead_unjoined(self):
        """`lane_west.end` 0.6 m in plan from `bridge.start` with no meeting between them, 0.4 m
        under the deck: `end_unjoined` with both; with the meeting, or 1.2 m off, nothing."""
        ends = [{"place": "bridge.start", "kind": "crossing", "at": (51000, 40000), "z": 21.72},
                {"place": "lane_west.end", "kind": "lane", "at": (50400, 40000), "z": 21.32},
                {"place": "lane_west.start", "kind": "lane", "at": (20000, 40000), "z": 20.0}]
        recs = CHK["unjoined"](ends, [], P)
        self.assertEqual(codes(recs), [("end_unjoined", "lane_west.end")])
        self.assertEqual(recs[0][5], {"distance": 0.6, "gap": 0.4, "other": "bridge.start"})
        self.assertEqual(recs[0][6], (50.4, 40.0))
        self.assertIn('ends_at = "bridge.start"', recs[0][4])
        self.assertEqual(CHK["unjoined"](ends, MEET, P), [])
        far = [dict(e, at=(49800, 40000)) if e["place"] == "lane_west.end" else e for e in ends]
        self.assertEqual(CHK["unjoined"](far, [], P), [])

    def test_ends_of_one_kind_are_not_judged(self):
        """Two lanes ending at one point (a fork is written by name, geometry.md 2.7), and a height
        not sampled: no record for the lanes, the gap None for a road."""
        ends = [{"place": "a.end", "kind": "lane", "at": (0, 0), "z": 1.0},
                {"place": "b.start", "kind": "lane", "at": (300, 0), "z": 1.0},
                {"place": "c.start", "kind": "road", "at": (0, 500), "z": None}]
        recs = CHK["unjoined"](ends, [], P)
        self.assertEqual(codes(recs), [("end_unjoined", "a.end"), ("end_unjoined", "b.start")])
        self.assertIsNone(recs[0][5]["gap"])
        self.assertEqual(CHK["params_of"]({"reach": 2})["reach"], 2.0)

    def test_joined_through_a_point_but_not_at_a_deck(self):
        """An import's trail and the parent's road both meeting the import's exit are joined
        through it; the parent's road and an import's deck meeting one point are not (a crossing's
        end is met only by a way naming it); two imports' ends are the imports' own."""
        ends = [{"place": "wild.south_trail.end", "kind": "trail", "at": (0, 0), "z": 5.0},
                {"place": "wild_road.end", "kind": "road", "at": (0, 0), "z": 5.0}]
        meet = [{"a": "wild.south_end", "b": "wild.south_trail.end", "gap": 0},
                {"a": "wild.south_end", "b": "wild_road.end", "gap": 0}]
        self.assertEqual(CHK["unjoined"](ends, meet, P), [])
        deck = [{"place": "creek_bridge.start", "kind": "crossing", "at": (0, 0), "z": 4.5},
                {"place": "creek_road.end", "kind": "road", "at": (0, 0), "z": 3.5}]
        meet = [{"a": "bridge_west", "b": "creek_bridge.start", "gap": 0},
                {"a": "bridge_west", "b": "creek_road.end", "gap": 0}]
        self.assertEqual(codes(CHK["unjoined"](deck, meet, P)), [("end_unjoined", "creek_road.end")])
        theirs = [dict(e, place="city." + e["place"]) for e in deck]
        self.assertEqual(CHK["unjoined"](theirs, [], P), [])

    def test_subjects_keep_the_import_scope(self):
        self.assertEqual(CHK["subject_of"]("townland.old_bridge", "c"), "feature.townland.old_bridge")
        self.assertEqual(CHK["subject_of"]("wild.south_trail.end", "c"), "feature.wild.south_trail")
        self.assertEqual(CHK["subject_of"]("bridge.start", "c"), "feature.bridge")
        self.assertEqual(CHK["subject_of"](None, "c"), "step.c")
        self.assertTrue(CHK["own"]("creek_bridge.start") and CHK["own"]("creek_bridge"))
        self.assertFalse(CHK["own"]("coast.harbour_bridge") or CHK["own"]("wild.south_trail.end"))

    def test_middle_is_half_the_arc_length(self):
        self.assertEqual(CHK["middle"]([(0, 0), (10000, 0), (10000, 10000)]), (10000, 0))
        self.assertEqual(CHK["middle"]([(0, 0), (3000, 4000)]), (1500, 2000))


if __name__ == "__main__":
    unittest.main()
