"""Tests of what examples/realm forced (docs/plan-kits.md K11): a plot levelled under a building
(lay.py rule 19) or under a court (court.py rule 9), and a doorway's sill that holds beside a way
(kinds.toml). Run from the repository root:

    PYTHONPATH=sdk/python python -m unittest discover -s lib/building -p 'test_ground.py'
"""

import os
import runpy
import shutil
import tomllib
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
# The two test modules whose helpers these tests reuse, read as lib/building reads its steps
# (library code imports only viasdk and the standard library).
TC = runpy.run_path(os.path.join(HERE, "test_court.py"))
TR = runpy.run_path(os.path.join(HERE, "test_row.py"))
L, K = TC["L"], TC["K"]
PE, KE = L["ParamError"], K["ParamError"]
KINDS = tomllib.load(open(os.path.join(HERE, "kinds.toml"), "rb"))["kind"]


class Plot(unittest.TestCase):
    """lay.py rule 19: `ground = "level"` on a building."""

    lay = TR["Foot"].lay
    rows = staticmethod(TR["Foot"].rows)
    RING = TR["Foot"].RING

    def test_the_plot_is_the_outline_moved_out_at_the_ground_the_floor_was_reckoned_from(self):
        out = self.lay(slope=0.08, ground="level")
        p = out["plot"]
        self.assertEqual(p["key"], "b_plot")
        self.assertEqual(p["region"], [(16500, 14500), (32100, 14500), (32100, 30100), (16500, 30100)])
        self.assertEqual(L["PLOT_APRON"], 1500)
        # the floor is the plinth over that level: one number, said once
        floor = out["storeys"][0]["level"]
        self.assertAlmostEqual(floor - p["level"], 0.3, places=3)

    def test_no_plot_without_the_word(self):
        self.assertNotIn("plot", self.lay(slope=0.08))

    def test_the_walls_foot_is_judged_against_the_level(self):
        """A rise of 0.2 m a metre over 12.6 m and a base 0.5 m deep: as the land lies the low
        side stands 1.56 m over the ground and the base stops short; on a plot the ground is the
        level all round, 0.3 m under the floor, and the base reaches it."""
        short = [n for n in self.lay(slope=0.2, base=500)["notes"] if n["code"] == "base_short"]
        self.assertEqual(len(short), 1)
        self.assertGreater(short[0]["hang"], 1.5)
        level = self.lay(slope=0.2, base=500, ground="level")
        self.assertFalse([n for n in level["notes"] if n["code"] in ("foot_open", "base_short")])

    def test_a_kit_with_no_base_still_shows_its_plinth_on_a_plot(self):
        """With no base, a slope says `foot_open` (warning); on a plot only the plinth shows, all
        round, which is `plinth_bare` (info), as on any level ground."""
        said = lambda out: [TR["L"]["foot_word"](n) for n in out["notes"] if n["code"] in ("foot_open", "base_short")]
        self.assertEqual(said(self.lay(slope=0.2, base=0)), ["foot_open"])
        self.assertEqual(said(self.lay(slope=0.2, base=0, ground="level")), ["plinth_bare"])

    def test_another_word_is_refused(self):
        with self.assertRaises(TR["L"]["ParamError"]) as c:
            self.lay(ground="flat")
        self.assertEqual(c.exception.key, "ground")
        self.assertIn("`ground` is 'flat'", c.exception.message)
        self.assertIn('`ground = "level"`', c.exception.hint)

    def test_a_wall_on_the_ground_takes_no_plot(self):
        a = {"storeys": 0, "storey_h": 2.4, "kit": "yard", "ground": "level"}
        rows = TC["yard_kit"]()
        with self.assertRaises(PE) as c:
            L["plan"]("y", self.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertEqual(c.exception.key, "ground")
        self.assertIn("no storeys", c.exception.message)
        self.assertIn("court", c.exception.hint)

    def test_the_step_writes_the_plot_with_its_level(self):
        ring = [(24350, 34000), (39650, 34000), (39650, 43300), (24350, 43300)]
        a = {"storeys": 1, "storey_h": 3.6, "plinth": 0.6, "kit": "@timber", "roof": "hip", "ground": "level"}
        s = TC["FakeStep"]({"h": (ring, a)}, item="h")
        L["generate"](s)
        self.assertFalse([x for x in s.said if x[0] == "error"], s.said)
        p = s.out.features["h_plot"]
        self.assertEqual((p["kind"], p["level"]), ("plot", 10.0))
        self.assertEqual(p["region"], [(22850, 32500), (41150, 32500), (41150, 44800), (22850, 44800)])
        self.assertNotIn("h_plot", {k for k, v in s.out.features.items() if v["kind"] != "plot"})

    def test_the_step_writes_none_without_the_word(self):
        ring = [(24350, 34000), (39650, 34000), (39650, 43300), (24350, 43300)]
        a = {"storeys": 1, "storey_h": 3.6, "plinth": 0.6, "kit": "@timber", "roof": "hip"}
        s = TC["FakeStep"]({"h": (ring, a)}, item="h")
        L["generate"](s)
        self.assertFalse([k for k, v in s.out.features.items() if v["kind"] == "plot"])


class CourtPlot(unittest.TestCase):
    """court.py rule 9: `ground = "level"` on a court."""

    def test_one_plot_under_the_yard_moved_out(self):
        b, _ = TC["court"](ground="level")
        yard, plot = b["c_yard"], b["c_plot"]
        (x0, y0), (x1, y1) = yard["ring"][0], yard["ring"][2]
        self.assertEqual(plot["ring"], [(x0 - 1500, y0 - 1500), (x1 + 1500, y0 - 1500), (x1 + 1500, y1 + 1500), (x0 - 1500, y1 + 1500)])
        self.assertTrue(plot["plot"])
        self.assertEqual(K["PLOT_APRON"], 1500)
        # the halls are returned as they were, and none asks for a plot of its own
        plain, _ = TC["court"]()
        self.assertEqual({k: v["ring"] for k, v in plain.items()}, {k: v["ring"] for k, v in b.items() if k != "c_plot"})
        self.assertFalse([k for k, v in b.items() if "attrs" in v and "ground" in v["attrs"]])

    def test_with_no_yard_the_plot_is_the_box_round_the_halls(self):
        b, _ = TC["court"](ground="level", yard=None)
        xs = [p[0] for k, v in b.items() if k != "c_plot" for p in v["ring"]]
        ys = [p[1] for k, v in b.items() if k != "c_plot" for p in v["ring"]]
        self.assertEqual(b["c_plot"]["ring"][0], (min(xs) - 1500, min(ys) - 1500))
        self.assertEqual(b["c_plot"]["ring"][2], (max(xs) + 1500, max(ys) + 1500))

    def test_a_turned_court_has_a_turned_plot(self):
        b, _ = TC["court"](ground="level", to=[42, 10])        # the axis: east
        yard, plot = b["c_yard"], b["c_plot"]
        xs, ys = [p[0] for p in yard["ring"]], [p[1] for p in yard["ring"]]
        self.assertEqual((min(p[0] for p in plot["ring"]), max(p[0] for p in plot["ring"])), (min(xs) - 1500, max(xs) + 1500))
        self.assertEqual((min(p[1] for p in plot["ring"]), max(p[1] for p in plot["ring"])), (min(ys) - 1500, max(ys) + 1500))

    def test_no_plot_without_the_word_and_another_word_is_refused(self):
        self.assertNotIn("c_plot", TC["court"]()[0])
        with self.assertRaises(KE) as c:
            TC["court"](ground="pad")
        self.assertEqual(c.exception.key, "ground")
        self.assertIn('`ground = "level"`', c.exception.hint)

    def test_inside_and_the_posts_a_level_is_read_at(self):
        ring = [(0, 0), (2000, 0), (2000, 1000), (0, 1000)]
        self.assertTrue(K["inside"](ring, (1000, 500)))
        self.assertTrue(K["inside"](ring, (2000, 500)))          # on an edge
        self.assertTrue(K["inside"](ring, (0, 0)))               # on a corner
        self.assertFalse(K["inside"](ring, (2001, 500)))
        self.assertFalse(K["inside"](ring, (1000, -1)))
        grid = {"extent_mm": [0, 0, 4000, 4000], "resolution_mm": 1000, "nx": 5, "ny": 5}
        pts = K["posts_in"](ring, grid)
        self.assertEqual(pts[:6], [(0, 0), (1000, 0), (2000, 0), (0, 1000), (1000, 1000), (2000, 1000)])
        self.assertEqual(pts[6:], ring)                           # then its corners
        self.assertEqual(K["posts_in"](ring, None), ring)
        # a ring past the world's edge reads only the posts the world has
        out = [(3000, 3000), (9000, 3000), (9000, 9000), (3000, 9000)]
        self.assertEqual(K["posts_in"](out, grid)[:4], [(3000, 3000), (4000, 3000), (3000, 4000), (4000, 4000)])

    def test_the_level_is_the_mean_of_the_land_to_the_millimetre(self):
        class Sloped(TC["FakeStep"]):
            def eval(self, formula, x, y):
                return [10.0 + 0.0001234 * v for v in x]

        ring = [(0, 0), (2000, 0), (2000, 1000), (0, 1000)]
        s = Sloped({})
        s.grid = {"extent_mm": [0, 0, 4000, 4000], "resolution_mm": 1000, "nx": 5, "ny": 5}
        # ten points: x of 0, 1000 and 2000 twice each, and the four corners
        mean = 10.0 + 0.0001234 * (2 * (0 + 1000 + 2000) + 0 + 2000 + 2000 + 0) / 10
        self.assertEqual(K["read_level"](s, ring), round(mean, 3))

        class NoField(TC["FakeStep"]):
            def fields(self, names, **kw):
                raise LookupError("no surface")
        self.assertEqual(K["read_level"](NoField({}), ring), 0.0)

        # a plot past the world's edge is read inside it only: its corners out there are left out
        # (F2's review: the corners were read, and the step raised `formula.outside_field`)
        class Edge(TC["FakeStep"]):
            def eval(self, formula, x, y):
                assert all(0 <= v <= 4000 for v in x + y), (x, y)
                return [10.0] * len(x)
        e = Edge({})
        e.grid = {"extent_mm": [0, 0, 4000, 4000], "resolution_mm": 1000, "nx": 5, "ny": 5}
        self.assertEqual(K["read_level"](e, [(3000, 1000), (6000, 1000), (6000, 2000), (3000, 2000)]), 10.0)

    def test_the_step_returns_the_plot_and_counts_the_buildings_alone(self):
        s = TC["CourtStep"]().run_step(ground="level")
        f = s.out.features
        self.assertEqual({k: v["kind"] for k, v in f.items()},
                         {"c_yard": "building", "c_hall": "building", "c_w0_l": "building", "c_w0_r": "building",
                          "c_plot": "plot"})
        self.assertEqual(f["c_plot"]["level"], 10.0)
        self.assertEqual(f["c_plot"]["region"][0], (f["c_yard"]["region"][0][0] - 1500, f["c_yard"]["region"][0][1] - 1500))
        self.assertEqual(s.said[-1][3], "1 courts, 4 buildings")
        self.assertEqual(s.said[-1][5], {"courts": 1, "buildings": 4})
        self.assertNotIn("ground", f["c_hall"])


class Kinds(unittest.TestCase):
    """kinds.toml: the plot, and a doorway's sill beside a way."""

    def test_a_plot_holds_the_stated_level_and_banks_outside(self):
        p = KINDS["plot"]
        self.assertEqual((p["authority"], p["requires"]), ("shaped", ["level"]))
        self.assertEqual(p["defaults"], {"grade": 0.3, "max_cut": 3})
        e = p["effect"]["level"]
        self.assertEqual(e["field"], "role:surface")
        self.assertEqual(e["value"], "= clamp(v, level - grade * d, level + grade * d)")
        self.assertEqual(e["max_change"], "= max_cut")
        self.assertEqual(e["order"], 1)                   # after a way's bed (0), before a sill (2), below
        self.assertEqual(list(p["effect"]), ["level"])    # it keeps nothing off: a kit's pieces stand on it

    def test_a_sill_is_shaped_and_laid_after_every_other_shaped_entry(self):
        d = KINDS["doorway"]
        self.assertEqual(d["authority"], "shaped")
        self.assertEqual(d["effect"]["sill"]["order"], 2)
        self.assertEqual(d["effect"]["sill"]["value"], "= sill_z")
        self.assertGreater(d["effect"]["sill"]["order"], KINDS["plot"]["effect"]["level"].get("order", 0))

    def test_building_and_court_take_the_word_and_a_row_does_not(self):
        self.assertEqual(KINDS["building"]["defaults"]["ground"], "")
        self.assertEqual(KINDS["court"]["defaults"]["ground"], "")
        self.assertNotIn("ground", KINDS["row"]["defaults"])
        self.assertIn("ground", L["ATTRS"])
        self.assertIn("ground", K["ATTRS"])
        self.assertNotIn("ground", K["PASSED"])


LAND = tomllib.load(open(os.path.join(HERE, "..", "land", "kinds.toml"), "rb"))["kind"]
C = runpy.run_path(os.path.join(HERE, "check.py"))


class PlotBank(unittest.TestCase):
    """The plot's own numbers (K11's review): its bank is lib/land's pad's, its radius covers the
    bank, its level is to the millimetre, and a building or a court passes its cut and bank on."""

    lay = TR["Foot"].lay
    rows = staticmethod(TR["Foot"].rows)
    RING = TR["Foot"].RING

    def test_the_bank_is_the_pads_with_the_level_stated(self):
        pad, plot = LAND["pad"], KINDS["plot"]
        self.assertEqual(plot["effect"]["level"]["mask"], pad["effect"]["level"]["mask"])
        self.assertEqual(plot["effect"]["level"]["value"],
                         pad["effect"]["level"]["value"].replace("under('mean')", "level"))
        self.assertEqual(plot["defaults"], pad["defaults"])
        self.assertEqual(plot["radius"], pad["radius"])

    def test_the_radius_reaches_past_the_fade(self):
        import re
        mask = KINDS["plot"]["effect"]["level"]["mask"]
        lo, hi = (float(v) for v in re.search(r"smoothstep\(([\d.]+), ([\d.]+), d\)", mask).groups())
        self.assertEqual((lo, hi), (10.0, 25.0))
        self.assertGreaterEqual(KINDS["plot"]["radius"], hi + 1)

    def test_the_court_level_is_to_the_millimetre_not_the_decimetre(self):
        class Odd(TC["FakeStep"]):
            def eval(self, formula, x, y):
                return [24.0823 for _ in x]
        s = Odd({})
        s.grid = {"extent_mm": [0, 0, 4000, 4000], "resolution_mm": 1000, "nx": 5, "ny": 5}
        self.assertEqual(K["read_level"](s, [(0, 0), (2000, 0), (2000, 1000), (0, 1000)]), 24.082)

    def test_the_court_step_writes_the_level_to_the_millimetre(self):
        class Odd(TC["FakeStep"]):
            def eval(self, formula, x, y):
                return [24.0823 for _ in x]
        a = {k: v for k, v in {**TC["COURT"], "ground": "level"}.items() if v is not None}
        s = Odd({"c": ([TC["GATE"]], a)})
        s.id = "courts"
        K["generate"](s)
        self.assertEqual(s.out.features["c_plot"]["level"], 24.082)

    def test_a_court_reads_its_level_only_where_there_is_no_surface(self):
        class Broken(TC["FakeStep"]):
            def fields(self, names, **kw):
                raise ValueError("not a missing field")
        with self.assertRaises(ValueError):
            K["read_level"](Broken({}), [(0, 0), (2000, 0), (2000, 1000), (0, 1000)])

    def test_the_floor_on_a_plot_is_its_plinth_over_the_level_exactly(self):
        """With no base, the only note on a plot is the plinth all round, and its hang is the
        plinth to the millimetre: the foot is the level `lay` read, not near it."""
        out = self.lay(slope=0.2, base=0, ground="level")
        notes = [n for n in out["notes"] if n["code"] in ("foot_open", "base_short")]
        self.assertEqual(len(notes), 1)
        self.assertAlmostEqual(notes[0]["hang"], 0.3, places=6)

    def test_a_building_passes_its_cut_and_bank_to_its_plot(self):
        self.assertEqual({k: v for k, v in self.lay(ground="level")["plot"].items() if k in ("grade", "max_cut")},
                         {"grade": 0.3, "max_cut": 3.0})
        p = self.lay(ground="level", max_cut=6, grade=0.5)["plot"]
        self.assertEqual((p["grade"], p["max_cut"]), (0.5, 6.0))
        with self.assertRaises(TR["L"]["ParamError"]) as c:
            self.lay(ground="level", max_cut="deep")
        self.assertEqual(c.exception.key, "max_cut")
        self.assertIn("max_cut", L["ATTRS"])
        self.assertIn("grade", L["ATTRS"])
        self.assertEqual((KINDS["building"]["defaults"]["max_cut"], KINDS["building"]["defaults"]["grade"]), (3, 0.3))

    def test_the_step_writes_the_plots_cut_and_bank(self):
        ring = [(24350, 34000), (39650, 34000), (39650, 43300), (24350, 43300)]
        a = {"storeys": 1, "storey_h": 3.6, "plinth": 0.6, "kit": "@timber", "roof": "hip", "ground": "level",
             "max_cut": 5.0}
        s = TC["FakeStep"]({"h": (ring, a)}, item="h")
        L["generate"](s)
        p = s.out.features["h_plot"]
        self.assertEqual((p["max_cut"], p["grade"]), (5.0, 0.3))

    def test_a_court_passes_its_cut_and_bank_to_its_plot(self):
        b, _ = TC["court"](ground="level")
        self.assertEqual((b["c_plot"]["max_cut"], b["c_plot"]["grade"]), (3.0, 0.3))
        b, _ = TC["court"](ground="level", max_cut=7, grade=0.4)
        self.assertEqual((b["c_plot"]["max_cut"], b["c_plot"]["grade"]), (7.0, 0.4))
        with self.assertRaises(KE) as c:
            TC["court"](ground="level", grade=-1)
        self.assertEqual(c.exception.key, "grade")
        s = TC["CourtStep"]().run_step(ground="level", max_cut=4)
        self.assertEqual((s.out.features["c_plot"]["max_cut"], s.out.features["c_plot"]["grade"]), (4.0, 0.3))
        self.assertIn("max_cut", K["ATTRS"])
        self.assertNotIn("max_cut", K["PASSED"])
        self.assertEqual((KINDS["court"]["defaults"]["max_cut"], KINDS["court"]["defaults"]["grade"]), (3, 0.3))


def feat(name, kind, ring, **attrs):
    return {"name": name, "kind": kind, "ring": ring, "path": ring, "attrs": attrs}


class PlotHeld(unittest.TestCase):
    """check.py rule 12: a building on a plot against the ground as it ended."""

    RING = [(10000, 10000), (20000, 10000), (20000, 18000), (10000, 18000)]
    PLOT = [(8500, 8500), (21500, 8500), (21500, 19500), (8500, 19500)]

    def features(self, **b):
        a = {"storeys": 1, "plinth": 0.3, "kit": "k", **b}
        return [feat("h", "building", self.RING, **a), feat("lay#h_plot", "plot", self.PLOT, level=10.0),
                feat("lay#h_s0", "storey", self.RING, level=10.3)]

    def test_a_building_on_its_plot_is_found_with_its_floor(self):
        (b, pl, floor), = C["on_plots"](self.features())
        self.assertEqual((b["name"], pl["name"], floor), ("h", "lay#h_plot", 10.3))

    def test_its_own_plot_first_then_the_first_that_holds_it(self):
        f = self.features() + [feat("a_plot", "plot", self.PLOT, level=9.0), feat("z_plot", "plot", self.PLOT, level=9.0)]
        self.assertEqual(C["on_plots"](f)[0][1]["name"], "lay#h_plot")
        f = [x for x in f if x["name"] != "lay#h_plot"]
        self.assertEqual(C["on_plots"](f)[0][1]["name"], "a_plot")

    def test_a_yard_a_building_half_off_and_one_with_no_storey_are_not_judged(self):
        self.assertEqual(C["on_plots"](self.features(storeys=0)), [])
        f = self.features()
        f[1] = feat("lay#h_plot", "plot", [(12000, 8500), (21500, 8500), (21500, 19500), (12000, 19500)], level=10.0)
        self.assertEqual(C["on_plots"](f), [])
        self.assertEqual(C["on_plots"]([x for x in self.features() if x["kind"] != "storey"]), [])

    def test_the_foot_is_read_every_half_metre_from_half_a_step_in(self):
        pts = C["foot_points"]([(0, 0), (2000, 0), (2000, 1000), (0, 1000)])
        self.assertEqual(pts[:5], [(250, 0), (750, 0), (1250, 0), (1750, 0), (2000, 250)])
        self.assertEqual(len(pts), 12)
        # a side of 1.8 m holds four points (3.6 halves, rounded), each 0.45 m apart
        self.assertEqual(C["foot_points"]([(0, 0), (1800, 0), (1800, 1000), (0, 1000)])[:4],
                         [(225, 0), (675, 0), (1125, 0), (1575, 0)])

    def judge(self, heights, base=0.0, by=None, **b):
        stands = C["on_plots"](self.features(**b))
        foot = {"h": [(x, y, heights(x, y)) for x, y in C["foot_points"](self.RING)]}
        return C["plots_lost"](stands, foot, {"k": base}, (lambda _b, _p: by) if by else None)

    def test_a_held_plot_says_nothing(self):
        self.assertEqual(self.judge(lambda x, y: 10.0), [])
        # the plinth shows all round, as the plot promised, and within 0.05 m of it
        self.assertEqual(self.judge(lambda x, y: 9.951), [])

    def test_ground_over_the_floor_is_said_with_its_length_and_what_shaped_it(self):
        (r,) = self.judge(lambda x, y: 10.5 if y >= 18000 else 10.0, by=("lay#g_plot", "plot", 1.8))
        sev, code, subject, msg, hint, data, pos = r
        self.assertEqual((sev, code, subject), ("warning", "plot_lost", "h"))
        self.assertAlmostEqual(data["over"], 0.2, places=6)
        self.assertEqual(data["under"], 0.0)
        self.assertEqual(data["length"], 10.0)
        self.assertEqual(data["by"], ["lay#g_plot", "plot"])
        self.assertIn("stands 0.200 m over the floor along 10.0 m", msg)
        self.assertIn("`lay#g_plot` (plot, 1.8 m off)", msg)
        self.assertIn("two plots", hint)
        self.assertEqual(pos[1], 18.0)

    def test_ground_below_what_the_base_and_the_plinth_reach_is_said(self):
        # a cut of 0.4 m on the south side: the floor stands 0.7 m over it, the base reaches 0.6 m
        (r,) = self.judge(lambda x, y: 9.6 if y <= 10000 else 10.0, base=0.6, by=("road1", "road", 3.0))
        data, msg, hint = r[5], r[3], r[4]
        self.assertAlmostEqual(data["under"], 0.1, places=6)
        self.assertEqual(data["reach"], 0.6)
        self.assertIn("falls 0.700 m below the floor, 0.100 m past the base's 0.6 m", msg)
        self.assertIn("keep `road1` off `lay#h_plot`", hint)
        # with a base that reaches 0.7 m, the cut is not said
        self.assertEqual(self.judge(lambda x, y: 9.6 if y <= 10000 else 10.0, base=0.7), [])
        # with no base the plinth is what the plot promised: 0.3 m
        (r,) = self.judge(lambda x, y: 9.9 if y <= 10000 else 10.0)
        self.assertIn("past the plinth's 0.3 m", r[3])
        self.assertIn("shaped after `lay` ran", r[3])

    def test_what_shaped_it_is_the_nearest_other_plot_or_shaped_kind_of_another_library(self):
        b = feat("h", "building", self.RING)
        cands = [{"name": "h", "kind": "building", "authority": "anchored", "rings": [self.RING]},
                 {"name": "lay#h_plot", "kind": "plot", "authority": "shaped", "rings": [self.PLOT]},
                 {"name": "lay#h_d0", "kind": "doorway", "authority": "shaped", "rings": [[(15000, 10000), (15000, 8000)]]},
                 {"name": "brook", "kind": "beck", "authority": "derived", "rings": [[(0, 9000), (40000, 9000)]]},
                 {"name": "lane", "kind": "road", "authority": "shaped", "rings": [[(0, 5000), (40000, 5000)]]},
                 {"name": "rock", "kind": "firm_pad", "authority": "anchored", "rings": [[(0, 0), (1000, 0), (1000, 1000), (0, 1000)]]}]
        self.assertEqual(C["nearest_cutter"](cands, b, (15000, 10000), "lay#h_plot"), ("lane", "road", 5.0))
        cands.append({"name": "lay#g_plot", "kind": "plot", "authority": "shaped",
                      "rings": [[(14000, 9000), (30000, 9000), (30000, 20000), (14000, 20000)]]})
        self.assertEqual(C["nearest_cutter"](cands, b, (15000, 10000), "lay#h_plot"), ("lay#g_plot", "plot", 0.0))
        self.assertIsNone(C["nearest_cutter"](cands[:4], b, (15000, 10000), "lay#h_plot"))
        # an anchored kind of another library is named too
        rock = [{"name": "rock", "kind": "firm_pad", "authority": "anchored",
                 "rings": [[(14000, 7000), (16000, 7000), (16000, 9000), (14000, 9000)]]}]
        self.assertEqual(C["nearest_cutter"](cands[:4] + rock, b, (15000, 10000), "lay#h_plot"), ("rock", "firm_pad", 1.0))
        # a neighbour whose name starts with this one's is still a neighbour
        h2 = [{"name": "lay#h_2_plot", "kind": "plot", "authority": "shaped",
               "rings": [[(14000, 9000), (30000, 9000), (30000, 20000), (14000, 20000)]]}]
        self.assertEqual(C["nearest_cutter"](cands[:4] + h2, b, (15000, 10000), "lay#h_plot"), ("lay#h_2_plot", "plot", 0.0))
        far = [{"name": "lane", "kind": "road", "authority": "shaped", "rings": [[(0, 50000), (40000, 50000)]]}]
        self.assertIsNone(C["nearest_cutter"](far, b, (15000, 10000), "lay#h_plot"))

    def test_the_explain_and_the_codes(self):
        self.assertIn("plot_lost", C["EXPLAIN"])
        self.assertEqual(C["FOOT_STEP"], 500)
        self.assertEqual(C["FOOT_SEEN"], 0.05)



class OffPlot(unittest.TestCase):
    """check.py rules 13 and 14 (the fix plan's F2): ground over a floor off a plot, and water at a
    wall, for every building."""

    RING = PlotHeld.RING

    def features(self, plot=False, **b):
        a = {"storeys": 1, "plinth": 0.3, "kit": "k", **b}
        f = [feat("h", "building", self.RING, **a), feat("lay#h_s0", "storey", self.RING, level=10.3)]
        return f + ([feat("lay#h_plot", "plot", PlotHeld.PLOT, level=10.0)] if plot else [])

    def foot(self, heights):
        return {"h": [(x, y, heights(x, y)) for x, y in C["foot_points"](self.RING)]}

    def test_every_building_with_a_ground_storey_stands_and_a_yard_does_not(self):
        (b, floor), = C["standing"](self.features())
        self.assertEqual((b["name"], floor), ("h", 10.3))
        self.assertEqual(C["standing"](self.features(storeys=0)), [])
        self.assertEqual(C["standing"](self.features(), skip={"h"}), [])

    def test_ground_over_the_floor_off_a_plot_is_said(self):
        """A house stepping with its land: 0.25 m over its floor along its uphill (north) wall."""
        (r,) = C["grounds_over"](C["standing"](self.features()), self.foot(lambda x, y: 10.55 if y >= 18000 else 10.0))
        sev, code, subject, msg, hint, data, pos = r
        self.assertEqual((sev, code, subject), ("warning", "ground_over", "h"))
        self.assertAlmostEqual(data["over"], 0.25, places=6)
        self.assertEqual(data["length"], 10.0)
        self.assertIn("the land shows through the floor", msg)
        self.assertIn('`ground = "level"`', hint)
        self.assertEqual(pos[1], 18.0)
        # within 0.05 m of the floor: nothing
        self.assertEqual(C["grounds_over"](C["standing"](self.features()), self.foot(lambda x, y: 10.349)), [])

    def test_a_wall_in_water_is_said_whether_or_not_the_floor_is_dry(self):
        st = C["standing"](self.features())
        ground = self.foot(lambda x, y: 10.0)
        # 0.2 m of water along the south wall: the floor (10.3) stands 0.1 m above it
        (r,) = C["in_water"](st, ground, self.foot(lambda x, y: 10.2 if y <= 10000 else 10.0))
        self.assertEqual((r[0], r[1], r[2]), ("warning", "in_water", "h"))
        self.assertAlmostEqual(r[5]["depth"], 0.2, places=6)
        self.assertEqual(r[5]["over"], 0.0)
        self.assertEqual(r[5]["length"], 10.0)
        self.assertIn("stands 0.100 m above the water", r[3])
        # 0.5 m of water: over the floor by 0.2 m, the room flooded
        (r,) = C["in_water"](st, ground, self.foot(lambda x, y: 10.5 if y <= 10000 else 10.0))
        self.assertAlmostEqual(r[5]["over"], 0.2, places=6)
        self.assertIn("the room is flooded", r[3])
        # dry: the water's surface is the ground
        self.assertEqual(C["in_water"](st, ground, ground), [])
        # 0.335 m of water: within 0.05 m of the floor, said as at it, not as a negative clearance
        (r,) = C["in_water"](st, ground, self.foot(lambda x, y: 10.335 if y <= 10000 else 10.0))
        self.assertIn("it stands at its floor (10.3 m), within 0.05 m", r[3])
        self.assertNotIn("-0.", r[3])

    def test_the_explain_and_the_codes(self):
        for code in ("ground_over", "in_water", "kit_param"):
            self.assertIn(code, C["EXPLAIN"])


class KitParam(unittest.TestCase):
    """check.py rule 15: a param no step of the library reads."""

    def kit(self, params):
        return [{"id": "bay", "taxon": "k", "tags": ["role:wall"], "bounds": [0, 0, 0, 3000, 300, 3000],
                 "params": params, "shared": {}}]

    def said(self, params):
        recs = C["judge"]([], [], self.kit(params), C["params_of"]({"kits": ["k"]}))
        return [r for r in recs if r[1] == "kit_param"]

    def test_a_misspelt_pitch_is_named_with_the_word_it_may_be(self):
        (r,) = self.said({"pich": 3.0})
        self.assertEqual((r[0], r[2], r[5]), ("warning", "bay", {"asset": "bay", "param": "pich"}))
        self.assertIn("did you mean `pitch`?", r[4])

    def test_a_key_far_from_any_is_named_without_a_guess(self):
        (r,) = self.said({"bay": True})
        self.assertNotIn("did you mean", r[4])

    def test_the_params_read_say_nothing(self):
        self.assertEqual(self.said({"pitch": 3.0, "reach": 0.3, "span": 2.0}), [])
        self.assertEqual(C["edits"]("pich", "pitch"), 1)


class Outside(unittest.TestCase):
    """lay.py rule 20: a building past the world's extent."""

    GRID = {"extent_mm": [0, 0, 40000, 32000]}

    def test_the_vertex_furthest_out(self):
        ring = [(30000, 8000), (45000, 8000), (45000, 12000), (30000, 12000)]
        self.assertEqual(L["past_extent"](ring, self.GRID), (5000, (45000, 8000)))

    def test_a_corner_out_on_both_axes_is_as_far_as_it_is(self):
        """F2's review: measured per axis, a corner 4 m out each way read 4 m; it is 5.657 m."""
        ring = [(30000, 20000), (44000, 20000), (44000, 36000), (30000, 36000)]
        self.assertEqual(L["past_extent"](ring, self.GRID), (5657, (44000, 36000)))

    def test_on_the_edge_is_inside(self):
        self.assertIsNone(L["past_extent"]([(0, 0), (40000, 0), (40000, 32000), (0, 32000)], self.GRID))
        self.assertIsNone(L["past_extent"]([(0, 0), (1, 0), (1, 1)], None))



VIA = os.environ.get("VIA_BIN") or shutil.which("via")
WET = """name = "wet"
extent = [0, 0, 48, 32]
resolution = 1
include = ["building"]

[field.height]                 # rising north, 0.1 a metre
role = "surface"
base = "= 10 + 0.1 * y"
{water}
[feature.uphill]               # on the slope: its uphill wall below the ground
kind    = "building"
region  = [[4, 22], [14, 22], [14, 30], [4, 30]]
storeys = 1

[feature.wet]                  # in the water, which stands at 12 m south of y = 20
kind    = "building"
region  = [[20, 4], [30, 4], [30, 10], [20, 10]]
storeys = 1

{past}
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
PAST = """
[feature.past]                 # its east wall 4 m past the extent
kind    = "building"
region  = [[40, 12], [52, 12], [52, 18], [40, 18]]
storeys = 1
"""
WATER = """
[field.water]
tags = ["water_level"]
base = "= max(f('height'), 12)"
"""


class Wiring(unittest.TestCase):
    """check.py's `lint` and lay.py's `generate` as the engine runs them (F2's review: the parts were
    tested, not their wiring): ground over a floor, a house in water, a building past the extent,
    and two fields tagged `water_level`."""

    def build(self, water, past=""):
        import subprocess
        import sys
        import tempfile
        self.assertIsNotNone(VIA, "set VIA_BIN")
        lib = os.path.dirname(HERE)
        with tempfile.TemporaryDirectory() as d:
            env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(d, ".config"))
            subprocess.run([VIA, "config", "program", "python", sys.executable], cwd=d, check=True,
                           env=env, stdout=subprocess.DEVNULL)
            with open(os.path.join(d, "world.toml"), "w") as f:
                f.write(WET.replace("{water}", water).replace("{past}", past))
            subprocess.run([VIA, "trust", "world.toml", "--yes", "--lib", lib], cwd=d, check=True,
                           env=env, stdout=subprocess.DEVNULL)
            r = subprocess.run([VIA, "build", "world.toml", "--lib", lib, "--out", "build/w"], cwd=d,
                               env=env, capture_output=True, text=True)
            return r.returncode, r.stdout + r.stderr

    def codes(self, text):
        return sorted(line.split("]")[0].split("[")[1] + " " + line.split("]")[1].split(":")[0].strip()
                      for line in text.splitlines() if line.startswith(("error[", "warning[")))

    def test_each_rule_says_its_building(self):
        code, text = self.build(WATER)
        self.assertEqual(code, 0)
        got = self.codes(text)
        self.assertIn('step.check.ground_over feature.uphill', got)
        self.assertIn('step.check.in_water feature.wet', got)
        self.assertIn("the room is flooded", text)

    def test_a_building_past_the_extent_is_an_error_not_a_traceback(self):
        """An error skips the lints (diagnostics.md 4), so this is a world of its own."""
        code, text = self.build(WATER, PAST)
        self.assertNotEqual(code, 0)
        self.assertIn('step.lay.outside feature.past', self.codes(text))
        self.assertNotIn("protocol.raised", text)
        self.assertIn("4 m past the world's extent", text)

    def test_two_water_fields_are_said_and_judge_nothing(self):
        _, text = self.build(WATER + WATER.replace("[field.water]", "[field.water2]"))
        got = self.codes(text)
        self.assertTrue(any(c.startswith("step.check.water_many") for c in got), got)
        self.assertFalse(any(c.startswith("step.check.in_water") for c in got), got)

    def test_with_no_water_field_rule_14_says_nothing(self):
        _, text = self.build("")
        got = self.codes(text)
        self.assertFalse(any("in_water" in c or "water_many" in c for c in got), got)
        self.assertIn('step.check.ground_over feature.uphill', got)


if __name__ == "__main__":
    unittest.main()
