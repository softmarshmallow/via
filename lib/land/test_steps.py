"""Tests for the land library's steps, weather.py and uplands.py, on small made-up grids.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/land -p 'test_*.py'

The engine's part (the tangent, the noise) is the engine's and tested there; these test the
arithmetic the steps do themselves, and what they return, with a stand-in for ``step``.
"""

import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
W = runpy.run_path(os.path.join(HERE, "weather.py"))
U = runpy.run_path(os.path.join(HERE, "uplands.py"))


class Weather(unittest.TestCase):
    def ramp(self, n=12, rise=3.0):
        return [[rise * i for i in range(n)] for _ in range(n)]

    def test_relax_keeps_material(self):
        h = self.ramp()
        out = W["relax"](h, 1.0, 30, 0.2)
        self.assertAlmostEqual(sum(map(sum, out)), sum(map(sum, h)), places=6)
        self.assertEqual(h, self.ramp())          # the input is untouched

    def test_relax_eases_steep_ground(self):
        out = W["relax"](self.ramp(), 1.0, 200, 0.2)
        steep = max(abs(r[i + 1] - r[i]) for r in out for i in range(len(r) - 1))
        self.assertLess(steep, 3.0)

    def test_gentle_ground_stays(self):
        h = self.ramp(rise=0.5)
        self.assertEqual(W["relax"](h, 1.0, 10, 0.2), h)

    def test_talus_is_zero_to_one(self):
        t = W["talus"]([[0.0, 2.0, -1.0], [0.5, 0.0, 4.0]])
        self.assertEqual(min(map(min, t)), 0.0)
        self.assertEqual(max(map(max, t)), 1.0)

    def test_upsample_reads_cells_exactly(self):
        c = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
        out = W["upsample"](c, 2, 5, 3)
        self.assertEqual(out[0][0], 1.0)
        self.assertEqual(out[0][2], 2.0)
        self.assertEqual(out[2][4], 6.0)
        self.assertEqual(out[1][1], 3.0)          # the middle of four cells

    def test_sample_takes_last_row_and_column(self):
        posts = [[float(10 * j + i) for i in range(5)] for j in range(4)]
        self.assertEqual(W["sample"](posts, 2), [[0.0, 2.0, 4.0], [20.0, 22.0, 24.0],
                                                 [30.0, 32.0, 34.0]])


class Uplands(unittest.TestCase):
    def bowl(self, n=9):
        return [[abs(i - 4) + abs(j - 4) + 0.1 * i for i in range(n)] for j in range(n)]

    def test_fill_leaves_every_inner_cell_draining(self):
        f = U["fill"](self.bowl())
        for j in range(1, 8):
            for i in range(1, 8):
                self.assertTrue(U["_lower"](f, i, j), (i, j))

    def test_area_leaves_by_the_edge(self):
        f = U["fill"](self.bowl())
        area = U["accumulate"](f, 2.0)
        n = len(f)
        edge = sum(area[j][i] for j in range(n) for i in range(n)
                   if i in (0, n - 1) or j in (0, n - 1))
        self.assertAlmostEqual(edge, n * n * 4.0, places=6)

    def test_stream_runs_down_to_the_edge(self):
        h = [[10.0 - j + 0.01 * abs(i - 4) for i in range(9)] for j in range(9)]   # falls north
        f = U["fill"](h)
        area = U["accumulate"](f, 1.0)
        line = U["stream"](f, area, 2.0)
        zs = [f[j][i] for i, j in line]
        self.assertEqual(zs, sorted(zs, reverse=True))
        self.assertEqual(line[-1][1], 8)

    def test_thin_keeps_ends_and_corners(self):
        pts = [(0, 0), (1, 0), (2, 0), (3, 0), (3, 1), (3, 2)]
        self.assertEqual(U["thin"](pts, 0.5), [(0, 0), (3, 0), (3, 2)])

    def test_inside_keeps_the_longest_run(self):
        line = [(-1, 0), (0, 0), (1, 1), (5, 5), (2, 2)]
        self.assertEqual(U["inside"](line, (0, 0, 3, 3)), [(0, 0), (1, 1)])

    def test_formula_rises_against_the_fall(self):
        p = U["Params"]({"fall": 180, "relief": 50})
        f = p.formula({"extent_mm": [0, 0, 100000, 200000]}, (0.0, -1.0))   # the engine's sin, cos of 180
        self.assertIn("(y - 100.0) * 1.0", f)
        self.assertIn("50.0 * (0.5 + ((x - 50.0) * ", f)
        self.assertIn("/ 200.0)", f)             # falls south: rises along y, over 200 m

    def test_log2_is_exact_where_it_can_be(self):
        for v, want in ((1.0, 0.0), (2.0, 1.0), (8.0, 3.0), (0.5, -1.0)):
            self.assertEqual(U["log2"](v), want)
        self.assertAlmostEqual(U["log2"](10.0), 3.321928094887362, places=14)

    def test_ticks(self):
        self.assertEqual(U["ticks"](9, 4), [0, 4, 8])
        self.assertEqual(U["ticks"](10, 4), [0, 4, 8, 9])


if __name__ == "__main__":
    unittest.main()


class Says:
    """A stand-in for ``step`` that only takes params and records what the step says."""

    def __init__(self, sid, params):
        self.id, self.params, self.errors = sid, params, []
        self.grid = {"extent_mm": [0, 0, 8000, 8000], "resolution_mm": 1000, "nx": 9, "ny": 9}

    def error(self, code, subject, message, hint, **kw):
        self.errors.append((code, subject, message, hint, kw))

    def fields(self, *a, **kw):
        raise AssertionError("a step with a bad param reads nothing")


class BadParams(unittest.TestCase):
    """A bad param is said at its key with the accepted range, never raised (principle 8)."""

    def check(self, module, sid, params, key, words):
        s = Says(sid, params)
        module["generate"](s)
        self.assertEqual(len(s.errors), 1)
        code, subject, message, hint, kw = s.errors[0]
        self.assertEqual(code, "params")
        self.assertEqual(subject, f"step.{sid}.params.{key}")
        self.assertIn(words, message)
        self.assertTrue(hint)
        self.assertTrue(kw.get("explain"))

    def test_weather_repose(self):
        self.check(W, "weather", {"repose": 95}, "repose", "above 0 and below 90")

    def test_weather_rate(self):
        self.check(W, "weather", {"rate": 0.9}, "rate", "at most 0.5")

    def test_uplands_start(self):
        self.check(U, "uplands", {"start": "flat"}, "start", "`start` is `flat`")

    def test_uplands_creep(self):
        self.check(U, "uplands", {"creep": 0.5}, "creep", "0 to 0.25")

    def test_uplands_cell(self):
        self.check(U, "uplands", {"cell": 0}, "cell", "above 0")
