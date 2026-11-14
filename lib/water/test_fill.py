"""Tests for fill.py, with a stand-in for the SDK's ``step``.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/water -p 'test_*.py'

The stand-in answers ``features``, ``fields``, ``spill`` and ``eval`` as protocol.md 4.3 and
derived.md 4 shape them. The engine's spill itself is tested by derived.md's cases d11 and d12.
"""

import os
import runpy
import unittest

FILL = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), "fill.py"))


class Named(list):
    name = "height"


class Out:
    def __init__(self):
        self.features = []

    def feature(self, key, kind, **attrs):
        self.features.append((key, kind, attrs))


class FakeStep:
    def __init__(self, tarns, level, dry, ground):
        self.tarns = tarns
        self.level, self.dry, self.ground = level, dry, ground
        self.out = Out()
        self.warnings = []
        self.calls = []

    def features(self, select, columns=()):
        assert select == "kind:tarn"
        cols = {"name": [], "geometry": [], "pieces": [0], "rings": [0], "ring_starts": [0],
                "coords": []}
        for k in ("freeboard", "fence", "max_depth"):
            cols[k], cols[k + "?"] = [], []
        for name, (x, y), attrs in self.tarns:
            cols["name"].append(name)
            cols["geometry"].append(0)
            cols["coords"].append([x, y])
            cols["ring_starts"].append(len(cols["coords"]))
            cols["rings"].append(len(cols["ring_starts"]) - 1)
            cols["pieces"].append(len(cols["rings"]) - 1)
            for k in ("freeboard", "fence", "max_depth"):
                cols[k].append(attrs.get(k, 0.0))
                cols[k + "?"].append(1 if k in attrs else 0)
        return cols

    def fields(self, names, view="full", window=None):
        self.calls.append(("fields", tuple(names), view))
        return [Named()]

    def spill(self, field, x, y, radius=None, view="full"):
        self.calls.append(("spill", field, tuple(radius), view))
        return {"level": [self.level] * len(x), "dry": [self.dry] * len(x)}

    def eval(self, formula, x=None, y=None, view="full"):
        self.calls.append(("eval", formula, view))
        return [self.ground] * len(x)

    def warning(self, code, subject, message, hint, **kw):
        self.warnings.append((code, subject, kw))


class Rule(unittest.TestCase):
    T = {"name": "tarn", "x": 338000, "y": 186000, "freeboard": 0.35, "fence": 0.0,
         "max_depth": 0.0}

    def test_level_less_freeboard(self):
        [(what, key, attrs)] = FILL["lake"](self.T, 10.0, False, 4.0, "height")
        self.assertEqual((what, key), ("lake", "tarn_lake"))
        self.assertEqual(attrs["water_height"], 9.65)
        self.assertEqual(attrs["where"], "= water_height - f('height')")
        self.assertEqual(attrs["holds"], [338.0, 186.0])

    def test_max_depth_caps(self):
        t = dict(self.T, max_depth=2.0)
        [(_, _, attrs)] = FILL["lake"](t, 10.0, False, 4.0, "height")
        self.assertEqual(attrs["water_height"], 6.0)

    def test_dry_point(self):
        [rec] = FILL["lake"](self.T, 4.0, True, 4.0, "height")
        self.assertEqual(rec[0], "dry")
        self.assertEqual(rec[2], {"level": 4.0, "ground": 4.0})

    def test_freeboard_below_ground_is_dry(self):
        [rec] = FILL["lake"](self.T, 4.2, False, 4.0, "height")
        self.assertEqual(rec[0], "dry")


class Generate(unittest.TestCase):
    def test_one_batch_head_view(self):
        s = FakeStep([("b", (1000, 2000), {"fence": 40.0}), ("a", (5000, 6000), {})],
                     12.0, False, 3.0)
        FILL["generate"](s)
        self.assertEqual([f[0] for f in s.out.features], ["a_lake", "b_lake"])
        self.assertTrue(all(f[1] == "lake" for f in s.out.features))
        spills = [c for c in s.calls if c[0] == "spill"]
        self.assertEqual(spills, [("spill", "height", (0, 40000), "head")])
        self.assertTrue(all(c[-1] == "head" for c in s.calls))

    def test_dry_warns_with_explain(self):
        s = FakeStep([("a", (5000, 6000), {})], 3.0, True, 3.0)
        FILL["generate"](s)
        self.assertEqual(s.out.features, [])
        [(code, subject, kw)] = s.warnings
        self.assertEqual((code, subject), ("dry", "feature.a"))
        self.assertIn("explain", kw)
        self.assertEqual(kw["position"], [5.0, 6.0])

    def test_no_tarns_reads_nothing(self):
        s = FakeStep([], 0, False, 0)
        FILL["generate"](s)
        self.assertEqual(s.calls, [])


if __name__ == "__main__":
    unittest.main()
