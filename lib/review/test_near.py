"""Tests for near.py, with a stand-in for the SDK's ``step``.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/review -p 'test_*.py'
"""

import os
import runpy
import unittest

N = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), "near.py"))


def feature_list(feats):
    cols = {"name": [], "kind": [], "geometry": [], "pieces": [0], "rings": [0],
            "ring_starts": [0], "coords": []}
    for name, kind, geom, pieces in feats:
        cols["name"].append(name)
        cols["kind"].append(kind)
        cols["geometry"].append(geom)
        for piece in pieces:
            for ring in piece:
                cols["coords"].extend([list(p) for p in ring])
                cols["ring_starts"].append(len(cols["coords"]))
            cols["rings"].append(len(cols["ring_starts"]) - 1)
        cols["pieces"].append(len(cols["rings"]) - 1)
    return cols


class FakeStep:
    def __init__(self, feats, insts, params):
        self.feats, self.insts, self.params = feats, insts, params
        self.warnings = []

    def features(self, select):
        kind = select[len("kind:"):]
        mine = [f for f in self.feats if f[1] == kind]
        if not mine:
            raise LookupError(f"`{select}` selects no feature")
        return feature_list(mine)

    def instances(self, select="*"):
        return {k: [r[k] for r in self.insts] for k in ("name", "asset", "x", "y")}

    def pack(self, select="*", columns=()):
        return {"id": ["house_low"] if select == "@building/house" else []}

    def warning(self, code, subject, message, hint, **kw):
        self.warnings.append((code, subject, kw))


ROAD = ("road", "road", 1, [[[(0, 0), (100000, 0)]]])
YARD = ("yard", "road", 2, [[[(0, 50000), (10000, 50000), (10000, 60000), (0, 60000)]]])


class Near(unittest.TestCase):
    def test_distance_to_path_region_point(self):
        shapes = [("p", 1, [[[(0, 0), (10000, 0)]]]), ("q", 0, [[[(0, 20000)]]])]
        self.assertEqual(N["nearest_shape"]((5000, 3000), shapes), (3000.0, "p"))
        self.assertEqual(N["nearest_shape"]((0, 19000), shapes), (1000.0, "q"))
        self.assertEqual(N["nearest_shape"]((5000, 55000), [YARD[:1] + YARD[2:]]), (0.0, "yard"))
        self.assertEqual(N["nearest_shape"]((1, 1), []), (None, None))

    def test_far_home_warned_near_home_not(self):
        s = FakeStep([ROAD], [
            {"name": "near_home", "asset": "house_low", "x": 50000, "y": 20000},
            {"name": "far_home", "asset": "house_low", "x": 50000, "y": 45000},
            {"name": "far_rock", "asset": "rock_a", "x": 50000, "y": 90000},
        ], {"assets": ["@building/house"], "kinds": ["road"], "within": 30})
        N["lint"](s)
        [(code, subject, kw)] = s.warnings
        self.assertEqual((code, subject), ("far", "instance.far_home"))
        self.assertEqual(kw["data"], {"distance": 45.0, "within": 30.0, "nearest": "road"})
        self.assertIn("explain", kw)

    def test_no_way_at_all(self):
        s = FakeStep([], [{"name": "hut", "asset": "hut", "x": 0, "y": 0}],
                     {"assets": ["hut"], "kinds": ["road"]})
        N["lint"](s)
        [(code, _, kw)] = s.warnings
        self.assertEqual(kw["data"]["nearest"], "")


if __name__ == "__main__":
    unittest.main()
