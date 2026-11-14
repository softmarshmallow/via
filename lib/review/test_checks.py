"""Tests for checks.py on a small made-up world, with a stand-in for the SDK's ``step``.

Run from the repository root with the SDK on the path:

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/review -p 'test_*.py'

(`-B`: no `__pycache__` in the library folder, whose digest is what `via trust` records.)

The stand-in answers ``fields``, ``features`` and ``instances`` as protocol.md 4.3 shapes them
and records ``info``. It is not the SDK: the plain-command transport is tested by protocol.md
case 40 through the engine.
"""

import os
import runpy
import unittest

CHECKS = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), "checks.py"))

N = 41  # posts along each axis, 1 m apart, over 0..40 m


class Cols(dict):
    """Columns as plain lists; ``tolist`` is not needed by the reader for lists."""


def feature_list(feats):
    """feats: (name, kind, geometry, pieces of rings of (x, y) mm[, number attributes]) ->
    protocol.md 4.3 columns, each attribute with its ``<attribute>?`` column."""
    cols = Cols(name=[], kind=[], geometry=[], pieces=[0], rings=[0], ring_starts=[0], coords=[])
    keys = sorted({k for f in feats if len(f) > 4 for k in f[4]})
    for k in keys:
        cols[k] = []
        cols[k + "?"] = []
    for name, kind, geom, pieces, *rest in feats:
        attrs = rest[0] if rest else {}
        for k in keys:
            cols[k].append(attrs.get(k, 0.0))
            cols[k + "?"].append(1 if k in attrs else 0)
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


def table(rows):
    return Cols({k: [r[k] for r in rows] for k in ("name", "asset", "by", "x", "y")})


class FakeStep:
    def __init__(self):
        self.grid = {"extent_mm": [0, 0, 40000, 40000], "resolution_mm": 1000, "nx": N, "ny": N}
        height = [[10 + 0.1 * i for i in range(N)] for _ in range(N)]
        # A level water surface over the posts 18 to 22 (lib/water: level across the channel), at
        # 12.5 m south of y = 20 m and 12.8 m from there north: one step of 0.3 m in the surface.
        # Dry posts hold the ground (build.md 6.4).
        water = [[max(h, 12.5 if j < 20 else 12.8) if abs(i - 20) <= 2 else h for i, h in enumerate(row)]
                 for j, row in enumerate(height)]
        keep = [[1.0 if 30 <= i <= 35 and 30 <= j <= 35 else 0.0 for i in range(N)] for j in range(N)]
        self._fields = {"role:surface": height, "tag:water_level": water, "role:keep_out": keep}
        beck = ("beck", "beck", 1, [[[(20000, 0), (20000, 40000)]]], {"width": 4.0})
        ford = ("ford", "ford", 0, [[[(20000, 20000)]]])
        west = ("lane_west", "lane", 1, [[[(0, 20000), (20000, 20000)]]])
        east = ("lane_east", "lane", 1, [[[(20000, 20000), (40000, 20000)]]])
        pad = ("cottage.ground", "pad", 2, [[[(5000, 5000), (9000, 5000), (9000, 9000), (5000, 9000)]]])
        self._features = {
            "kind:beck": [beck],
            "kind:ford": [ford],
            "kind:lane": [east, west],
            "*": [beck, pad, ford, east, west],
        }
        cottage = {"name": "cottage", "asset": "house_low", "by": "", "x": 7000, "y": 7000}
        t1 = {"name": "trees#s7000_a", "asset": "tree_oak", "by": "trees", "x": 32000, "y": 32000}
        t2 = {"name": "trees#s7000_b", "asset": "tree_oak", "by": "trees", "x": 10000, "y": 30000}
        self._instances = {"*": [cottage, t1, t2], "@plant/tree": [t1, t2]}
        self.records = []

    def fields(self, names):
        return [self._fields[n] for n in names]

    def features(self, select):
        return feature_list(self._features[select])

    def instances(self, select):
        return table(self._instances.get(select, []))

    def info(self, **kw):
        self.records.append(dict(kw, severity="info"))

    def warning(self, **kw):
        self.records.append(dict(kw, severity="warning"))


class ChecksTest(unittest.TestCase):
    def setUp(self):
        self.step = FakeStep()
        CHECKS["lint"](self.step)
        self.by = {(r["code"], r["subject"]): r for r in self.step.records}

    def value(self, code, subject):
        return self.by[(code, subject)]["data"]["value"]

    def test_every_measure(self):
        self.assertEqual(self.value("lane_gap", "feature.lane_west"), 0)
        self.assertEqual(self.value("lane_gap", "feature.lane_east"), 0)
        self.assertEqual(self.value("approach_grade", "feature.ford"), 0.1)
        self.assertEqual(self.value("ford_depth", "feature.ford"), 0.8)
        self.assertEqual(self.value("water_step", "feature.beck"), 0.3)
        self.assertEqual(self.value("cottage_relief", "instance.cottage"), 0.4)
        self.assertEqual(self.value("trees_in_clearing", "scatter.trees"), 1)

    def test_positions(self):
        self.assertEqual(self.by[("lane_gap", "feature.lane_west")]["position"], [20.0, 20.0])
        self.assertEqual(self.by[("ford_depth", "feature.ford")]["position"], [20.0, 20.0])
        self.assertEqual(self.by[("water_step", "feature.beck")]["position"], [18.0, 19.5])
        self.assertEqual(self.by[("cottage_relief", "instance.cottage")]["position"], [7.0, 7.0])
        self.assertNotIn("position", self.by[("trees_in_clearing", "scatter.trees")])

    def test_first_of_each_code_explains(self):
        seen = set()
        for r in self.step.records:
            self.assertEqual("explain" in r, r["code"] not in seen, r)
            self.assertTrue(r["hint"])
            seen.add(r["code"])
        self.assertEqual(len(seen), 6)

    def test_same_records_twice(self):
        again = FakeStep()
        CHECKS["lint"](again)
        self.assertEqual(again.records, self.step.records)

    def test_water_step_reach_is_the_becks_width(self):
        # A line at x = 24 m lies 2 m from the nearest wet post (posts 18 to 22); a beck 1 m wide
        # reaches 1 m, so no wet pair is near it, and a beck without `width` is not measured.
        for attrs in ({"width": 1.0}, {}):
            step = FakeStep()
            beck = ("beck", "beck", 1, [[[(24000, 0), (24000, 40000)]]], attrs)
            step._features["*"] = [beck] + [f for f in step._features["*"] if f[1] != "beck"]
            CHECKS["lint"](step)
            codes = {r["code"] for r in step.records}
            self.assertNotIn("water_step", codes, attrs)

    def test_all_info_when_every_field_is_there(self):
        self.assertEqual({r["severity"] for r in self.step.records}, {"info"})

    def test_world_without_water(self):
        # A field the world lacks is said, per subject, as an `unmeasured` warning naming the
        # measure; the measures that do not need it are still taken.
        step = FakeStep()
        del step._fields["tag:water_level"]  # the stand-in then raises KeyError, a LookupError
        CHECKS["lint"](step)
        codes = {r["code"] for r in step.records}
        self.assertNotIn("ford_depth", codes)
        self.assertNotIn("water_step", codes)
        self.assertIn("approach_grade", codes)
        warned = {(r["subject"], r["data"]["measure"]): r for r in step.records
                  if r["code"] == "unmeasured"}
        self.assertEqual(sorted(warned), [("feature.beck", "water_step"), ("feature.ford", "ford_depth")])
        for r in warned.values():
            self.assertEqual(r["severity"], "warning")
            self.assertIn("tag:water_level", r["message"])
            self.assertTrue(r["hint"])
            self.assertIn("position", r)
        self.assertEqual(warned[("feature.ford", "ford_depth")]["position"], [20.0, 20.0])
        first = [r for r in step.records if r["code"] == "unmeasured"][0]
        self.assertIn("explain", first)

    def test_world_without_surface_or_keep_out(self):
        step = FakeStep()
        del step._fields["role:surface"]
        del step._fields["role:keep_out"]
        CHECKS["lint"](step)
        warned = sorted((r["subject"], r["data"]["measure"]) for r in step.records
                        if r["code"] == "unmeasured")
        self.assertEqual(warned, sorted([
            ("feature.ford", "approach_grade"),
            ("feature.ford", "ford_depth"),
            ("feature.beck", "water_step"),
            ("instance.cottage", "cottage_relief"),
            ("scatter.trees", "trees_in_clearing"),
        ]))
        self.assertEqual({r["code"] for r in step.records if r["severity"] == "info"}, {"lane_gap"})

    def test_trees_in_clearing_counts_only_scatters(self):
        # A pin standing in keep_out is the build's place.keep_out (rules.md 8.3), not this
        # measure: the scatter keeps its one record and its count.
        step = FakeStep()
        pin = {"name": "oak", "asset": "tree_oak", "by": "pin.oak", "x": 33000, "y": 33000}
        step._instances["*"].append(pin)
        step._instances["@plant/tree"].append(pin)
        CHECKS["lint"](step)
        got = [(r["subject"], r["data"]["value"]) for r in step.records if r["code"] == "trees_in_clearing"]
        self.assertEqual(got, [("scatter.trees", 1)])

    def test_trees_in_clearing_counts_the_fade(self):
        # keep_out only multiplies the chance (rules.md 7.1): a tree where it is 0.9 or more is
        # counted (the reported tree stood at 0.988), one in the fade below 0.9 is not.
        for value, want in ((0.988, 2), (0.9, 2), (0.74, 1)):
            step = FakeStep()
            step._fields["role:keep_out"][30][10] = value   # under trees#s7000_b at (10, 30) m
            CHECKS["lint"](step)
            r = [r for r in step.records if r["code"] == "trees_in_clearing"]
            self.assertEqual([x["data"]["value"] for x in r], [want], value)

    def test_hints_name_the_edit(self):
        hints = CHECKS["HINTS"]
        self.assertIn("`bank_grade`", hints["water_step"])
        self.assertIn("`verge`", hints["trees_in_clearing"])

    def test_engine_faults_fail_the_step(self):
        # An error the engine answers is not a missing field: it propagates (protocol.md 3.4), so
        # the step fails and no measure is quietly dropped.
        from viasdk import RpcError

        def refuse(*_a, **_k):
            raise RpcError(-32002, "blob missing")

        for read in ("fields", "features", "instances"):
            step = FakeStep()
            setattr(step, read, refuse)
            with self.assertRaises(RpcError, msg=read):
                CHECKS["lint"](step)


if __name__ == "__main__":
    unittest.main()


class MoreTest(unittest.TestCase):
    """The valley's measures: standing water, trees on ways and crowded views."""

    def grid(self):
        return CHECKS["Grid"]({"extent_mm": [0, 0, 40000, 40000], "resolution_mm": 1000,
                               "nx": N, "ny": N})

    def test_lake_area_depth_islands_shore(self):
        height = [[10.0] * N for _ in range(N)]
        water = [[10.0] * N for _ in range(N)]
        for j in range(10, 21):
            for i in range(10, 21):
                water[j][i] = 11.5
        height[15][15] = 12.0   # an islet stands above the water
        water[15][15] = 12.0
        outer = [(10000, 10000), (20000, 10000), (20000, 20000), (10000, 20000)]
        hole = [(15000, 15000), (15000, 16000), (16000, 16000), (16000, 15000)]
        lake = {"name": "tarn", "kind": "lake", "geometry": 2, "rings": [[outer, hole]],
                "attrs": {}}
        recs = CHECKS["measure_more"](self.grid(), height, water, [lake], [], [])
        by = {r[0]: r[2] for r in recs}
        self.assertEqual(by["water_islands"], 1)
        self.assertEqual(by["water_depth"], 1.5)
        self.assertEqual(by["water_shore"], 40.0)
        self.assertEqual(by["water_area"], 120.0)   # 121 posts, the islet dry

    def test_trees_on_way_and_crowded_view(self):
        way = {"name": "track", "kind": "trail", "geometry": 1,
               "rings": [[[(0, 5000), (40000, 5000)]]], "attrs": {"width": 2.0}}
        mark = {"name": "look", "kind": "bookmark", "geometry": 0,
                "rings": [[[(30000, 30000)]]], "attrs": {}}
        trees = [{"name": "t#a", "asset": "tree_oak", "by": "trees", "x": 10000, "y": 5500},
                 {"name": "t#b", "asset": "tree_oak", "by": "trees", "x": 10000, "y": 9000},
                 {"name": "t#c", "asset": "tree_oak", "by": "trees", "x": 31000, "y": 31000}]
        recs = CHECKS["measure_more"](self.grid(), None, None, [way, mark], trees, [])
        by = {r[0]: r[2] for r in recs}
        self.assertEqual(by["trees_on_way"], 1)
        self.assertEqual(by["crowded_view"], 1.41)

