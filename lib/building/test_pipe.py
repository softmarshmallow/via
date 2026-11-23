"""Tests for pipe.py (a pipe's legs, the pieces at its points and ends, the fit, supports, the
step), and for a port: a variant of the bay with a mount, which lay.py lays in its run as any
variant (rule 8).

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The kits are examples/works' (packs/works.toml), as `pack.query` gives their rows: `pipe`, a
straight piece of 2.0 m, a slip piece of 0.5 m, an elbow, a riser of 3.0 m, a flange, a cap and a
support for a pipe 4.5 m up, each end piece taking 0.5 m of a leg; `shed`, laid by posts on a
6.0 m bay; `boiler`, walls at a pitch of 3.0 m.
"""

import json
import math
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
P = runpy.run_path(os.path.join(HERE, "pipe.py"))
L = runpy.run_path(os.path.join(HERE, "lay.py"))
PE = P["ParamError"]

SHARED = {"connector": "pipe", "section": 0.3, "support": 4.0}


def prow(id_, role, shared=SHARED, bounds=None, **params):
    return {"id": id_, "tags": [f"role:{role}"], "bounds": bounds, "params": dict(params), "shared": shared}


def pipe_rows(drop=(), swap=None, shared=SHARED):
    swap = swap or {}
    rows = [prow("pipe", "pipe", pitch=2.0), prow("pipe_slip", "slip", pitch=0.5),
            prow("pipe_elbow", "elbow", angle=90, reach=0.5), prow("pipe_riser", "riser", rise=3.0, reach=0.5),
            prow("pipe_flange", "flange", reach=0.5), prow("pipe_cap", "cap", reach=0.5),
            prow("pipe_support", "support", height=4.5)]
    return [dict(swap.get(r["id"], r), shared=shared) for r in rows if r["id"] not in drop]


def kit(**kw):
    return P["kit_facts"](pipe_rows(**kw), "pipe")


# examples/works' main pipe: east out of the shed, up and over, down, south, east into the boiler house.
MAIN = [(32400, 29200), (36400, 29200), (46400, 29200), (48400, 29200), (48400, 23200), (50400, 23200)]
HEIGHTS = [4.5, [4.5, 7.5], [7.5, 4.5], 4.5, 4.5, 4.5]


def plan(pts=MAIN, heights=HEIGHTS, k=None, ground=10.0, **attrs):
    return P["plan_pipe"]("m", pts, {"heights": heights, **attrs}, k or kit(), ground)


def by_key(out, what):
    return {x["key"]: x for x in out[what]}


def error(test, *a, **kw):
    with test.assertRaises(PE) as c:
        plan(*a, **kw)
    return c.exception


class Facts(unittest.TestCase):
    """What the step reads of a pipe kit."""

    def test_each_role_and_its_lengths_in_millimetres(self):
        k = kit()
        self.assertEqual((k["pipe"]["id"], k["pipe"]["pitch"], k["slip"]["pitch"]), ("pipe", 2000, 500))
        self.assertEqual([(e["id"], e["angle"], e["reach"]) for e in k["elbows"]], [("pipe_elbow", 90, 500)])
        self.assertEqual([(r["id"], r["rise"], r["reach"]) for r in k["risers"]], [("pipe_riser", 3000, 500)])
        self.assertEqual((k["flange"]["reach"], k["cap"]["reach"], k["support"]["height"], k["spacing"]), (500, 500, 4500, 4000))

    def test_a_piece_with_no_pitch_is_as_long_as_its_box_and_takes_no_reach(self):
        k = kit(swap={"pipe": prow("pipe", "pipe", bounds=[-900, -150, -150, 900, 150, 150]),
                      "pipe_cap": prow("pipe_cap", "cap")})
        self.assertEqual((k["pipe"]["pitch"], k["cap"]["reach"]), (1800, 0))

    def test_of_two_straight_pieces_the_first_by_name_is_the_kits(self):
        rows = pipe_rows() + [prow("a_pipe", "pipe", pitch=1.0)]
        self.assertEqual(P["kit_facts"](rows, "pipe")["pipe"]["id"], "a_pipe")

    def test_a_kit_may_hold_two_risers_and_two_elbows(self):
        rows = pipe_rows() + [prow("riser_6", "riser", rise=6.0, reach=0.5), prow("elbow_45", "elbow", angle=45, reach=0.3)]
        k = P["kit_facts"](rows, "pipe")
        self.assertEqual(([r["rise"] for r in k["risers"]], [e["angle"] for e in k["elbows"]]), ([3000, 6000], [45, 90]))

    def test_no_straight_piece_is_refused(self):
        with self.assertRaises(PE) as c:
            kit(drop=("pipe",))
        self.assertIn("no `role:pipe` piece", c.exception.message)

    def test_a_taxon_with_no_piece_is_refused(self):
        with self.assertRaises(PE) as c:
            P["kit_facts"]([], "service/none")
        self.assertEqual(c.exception.message, "no piece of `service/none` is in the pack")

    def test_an_elbow_says_its_angle_and_a_riser_its_rise(self):
        for id_, role, word in (("pipe_elbow", "elbow", "angle"), ("pipe_riser", "riser", "rise")):
            with self.assertRaises(PE) as c:
                kit(swap={id_: prow(id_, role, reach=0.5)})
            self.assertIn(f"says no `params.{word}`", c.exception.message)

    def test_a_number_that_is_none_is_refused_with_its_piece(self):
        for bad in (0, -1, True, "2"):
            with self.assertRaises(PE) as c:
                kit(swap={"pipe": prow("pipe", "pipe", pitch=bad)})
            self.assertIn("`pipe` of `pipe` says `pitch = ", c.exception.message)

    def test_the_spacing_of_supports_is_the_kits_or_none(self):
        self.assertIsNone(kit(shared={"section": 0.3})["spacing"])
        for bad in (0, -4, True, "4"):
            with self.assertRaises(PE) as c:
                kit(shared={"support": bad})
            self.assertIn("shared.support", c.exception.message)


class Written(unittest.TestCase):
    """Rule 1: the points and their heights."""

    def test_a_height_is_one_number_or_arriving_and_leaving(self):
        self.assertEqual(P["heights_of"]([4.5, [4.5, 7.5], 7.5], 3), [(4500, 4500), (4500, 7500), (7500, 7500)])

    def test_one_height_for_each_point(self):
        e = error(self, heights=[4.5, 4.5])
        self.assertEqual((e.key, e.message), ("heights", "`heights` has 2 entries for a pipe of 6 points"))
        self.assertIn("none entries", error(self, heights=None).message)

    def test_a_height_that_is_no_number_is_refused_by_its_place(self):
        for bad in ("4.5", [4.5], [4.5, 5, 6], True, [4.5, "x"], float("nan")):
            e = error(self, pts=MAIN[:2], heights=[4.5, bad])
            self.assertTrue(e.message.startswith("`heights[1]` is "), e.message)

    def test_the_points_after_the_first_are_metres(self):
        self.assertEqual(P["points_of"]([[36.4, 29.2], [46, 29]]), [(36400, 29200), (46000, 29000)])
        for bad in (None, [], [[1]], [[1, 2, 3]], [[1, "2"]], [1, 2], [[True, 2]]):
            with self.assertRaises(PE) as c:
                P["points_of"](bad)
            self.assertEqual(c.exception.key, "to")

    def test_a_pipe_is_two_points_or_more(self):
        self.assertEqual(error(self, pts=MAIN[:1], heights=[4.5]).key, "to")

    def test_the_first_and_the_last_point_have_one_height(self):
        e = error(self, pts=MAIN[:2], heights=[[4.5, 7.5], 7.5])
        self.assertEqual(e.message, "`heights[0]` gives the pipe's first point two heights")
        e = error(self, pts=MAIN[:2], heights=[4.5, [4.5, 7.5]])
        self.assertEqual(e.message, "`heights[1]` gives the pipe's last point two heights")


class Legs(unittest.TestCase):
    """Rule 2: level legs, each a path with its surface."""

    def test_a_leg_between_each_two_points_at_its_height_over_the_ground(self):
        legs = plan()["legs"]
        self.assertEqual([x["key"] for x in legs], ["m_l0", "m_l1", "m_l2", "m_l3", "m_l4"])
        self.assertEqual([x["level"] for x in legs], [14.5, 17.5, 14.5, 14.5, 14.5])
        self.assertEqual([x["level"] for x in plan(ground=12.345)["legs"]][:2], [16.845, 19.845])

    def test_a_leg_names_what_it_starts_and_ends_at_and_its_path_holds_the_rest(self):
        legs = by_key(plan(), "legs")
        # up at point 1: the upper leg starts on the riser's top; down at point 2: it ends on that riser's top
        self.assertEqual((legs["m_l1"]["starts_at"], legs["m_l1"]["ends_at"], legs["m_l1"]["path"]), ("m_r1.top", "m_r2.top", []))
        # the lower legs name no riser: the riser stands on them
        self.assertEqual((legs["m_l0"]["starts_at"], legs["m_l0"]["ends_at"], legs["m_l0"]["path"]),
                         (None, None, [(32400, 29200), (36400, 29200)]))
        self.assertEqual((legs["m_l2"]["starts_at"], legs["m_l2"]["ends_at"]), (None, None))
        # a leg after an elbow starts at the end of the leg before it
        self.assertEqual((legs["m_l3"]["starts_at"], legs["m_l3"]["path"]), ("m_l2.end", [(48400, 23200)]))
        self.assertEqual(legs["m_l4"]["starts_at"], "m_l3.end")

    def test_a_leg_that_climbs_is_refused_with_the_riser_that_would_do_it(self):
        e = error(self, pts=MAIN[:3], heights=[4.5, 4.5, 5])
        self.assertEqual(e.message, "leg 1, from point 1 to point 2, climbs from 4.5 m to 5 m: a leg is level")
        # point 2 is the pipe's last and can hold no riser: the hint writes it at point 1
        self.assertIn("`heights[1] = [4.5, 5]` is a riser there", e.hint)
        self.assertIn("The kit's riser makes 3 m", e.hint)

    def test_two_points_at_one_place_are_refused(self):
        e = error(self, pts=[MAIN[0], MAIN[1], MAIN[1], MAIN[2]], heights=[4.5, 4.5, 7.5, 7.5])
        self.assertEqual(e.message, "points 1 and 2 of the pipe are one place")
        self.assertIn("`heights[1] = [arriving, leaving]`", e.hint)

    def test_a_point_in_line_at_one_height_holds_no_piece_and_the_legs_own_their_meeting(self):
        out = plan(pts=[(0, 0), (4000, 0), (9000, 0)], heights=[4.5, 4.5, 4.5])
        self.assertEqual(sorted(i["key"] for i in out["instances"] if "_p" not in i["key"]), ["m_end0", "m_end1"])
        self.assertEqual(by_key(out, "legs")["m_l1"]["starts_at"], "m_l0.end")
        runs = by_key(out, "runs")
        self.assertEqual((runs["m_l0_s"]["count"], runs["m_l0_s"]["span"]), (1, [0.125, 0.625]))     # 0.5 for the flange, none at the joint
        self.assertEqual(runs["m_l0_f"]["span"], [0.625, 1.0])


class Points(unittest.TestCase):
    """Rule 3: an elbow at a turn, a riser at a change of height."""

    def test_an_elbow_stands_on_the_leg_that_arrives_turned_to_both_legs(self):
        insts = by_key(plan(), "instances")
        # east then south: a turn to the right, the piece's back to the west and its right to the south
        self.assertEqual((insts["m_e3"]["asset"], insts["m_e3"]["at"], insts["m_e3"]["face"], insts["m_e3"]["surface"]),
                         ("pipe_elbow", (48400, 29200), 90.0, "m_l2.line"))
        # south then east: a turn to the left, its right to the north and its back to the east
        self.assertEqual((insts["m_e4"]["face"], insts["m_e4"]["surface"]), (270.0, "m_l3.line"))

    def test_every_quarter_turn_is_the_one_piece_turned(self):
        for (dx, dy), (ex, ey), face in (((0, 1), (1, 0), 0.0), ((0, 1), (-1, 0), 90.0), ((1, 0), (0, 1), 180.0),
                                         ((-1, 0), (0, 1), 270.0), ((0, -1), (1, 0), 270.0), ((0, -1), (-1, 0), 180.0)):
            a, b = (4000 * dx, 4000 * dy), (4000 * dx + 4000 * ex, 4000 * dy + 4000 * ey)
            out = plan(pts=[(0, 0), a, b], heights=[4.5, 4.5, 4.5])
            self.assertEqual(by_key(out, "instances")["m_e1"]["face"], face, ((dx, dy), (ex, ey)))

    def test_a_turn_the_kit_has_no_elbow_for_is_refused_with_the_angles_it_has(self):
        e = error(self, pts=[(0, 0), (4000, 0), (8000, 4000)], heights=[4.5, 4.5, 4.5])
        self.assertEqual(e.message, "the pipe turns 45 degrees at point 1: `pipe` has an elbow of 90 degrees")
        self.assertEqual(e.hint, "turn by what the kit's elbow makes (90 degrees)")
        e = error(self, pts=[(0, 0), (4000, 0), (4000, 4000)], heights=[4.5, 4.5, 4.5], k=kit(drop=("pipe_elbow",)))
        self.assertIn("has no `role:elbow` piece", e.message)

    def test_a_turn_within_a_twentieth_of_a_degree_of_the_elbows_is_the_elbows(self):
        out = plan(pts=[(0, 0), (40000, 0), (40030, 40000)], heights=[4.5, 4.5, 4.5])       # 0.043 degrees off square
        self.assertIn("m_e1", by_key(out, "instances"))
        self.assertEqual(error(self, pts=[(0, 0), (40000, 0), (40050, 40000)], heights=[4.5, 4.5, 4.5]).key, "to")

    def test_a_riser_up_stands_on_the_leg_below_facing_on(self):
        r = by_key(plan(), "instances")["m_r1"]
        self.assertEqual((r["asset"], r["at"], r["face"], r["surface"]), ("pipe_riser", (36400, 29200), 90.0, "m_l0.line"))

    def test_a_riser_down_stands_on_the_leg_below_facing_back(self):
        r = by_key(plan(), "instances")["m_r2"]
        self.assertEqual((r["at"], r["face"], r["surface"]), ((46400, 29200), 270.0, "m_l2.line"))

    def test_a_height_the_riser_does_not_make_is_refused_with_what_it_makes(self):
        e = error(self, pts=MAIN[:3], heights=[4.5, [4.5, 6.5], 6.5])
        self.assertEqual(e.message, "the pipe climbs 2 m at point 1: `pipe` has a riser of 3 m")
        self.assertEqual(e.hint, "climb by what the kit's riser makes (3 m)")
        e = error(self, pts=MAIN[:3], heights=[4.5, [4.5, 7.5], 7.5], k=kit(drop=("pipe_riser",)))
        self.assertIn("has no `role:riser` piece", e.message)

    def test_a_riser_where_the_pipe_turns_is_refused(self):
        e = error(self, pts=[(0, 0), (4000, 0), (4000, 4000)], heights=[4.5, [4.5, 7.5], 7.5])
        self.assertEqual(e.message, "the pipe climbs and turns at point 1")

    def test_each_piece_at_a_point_takes_its_reach_of_both_legs(self):
        runs = by_key(plan(), "runs")
        # leg 1 is 10 m between two risers: 0.5 from each, 9 m of pieces
        self.assertEqual((runs["m_l1_s"]["count"], runs["m_l1_s"]["span"]), (4, [0.05, 0.85]))
        self.assertEqual((runs["m_l1_f"]["count"], runs["m_l1_f"]["span"]), (2, [0.85, 0.95]))


class Ends(unittest.TestCase):
    """Rule 4: a plug or a cap on each end."""

    def test_a_flange_on_each_end_facing_out_of_the_pipe(self):
        insts = by_key(plan(), "instances")
        self.assertEqual((insts["m_end0"]["asset"], insts["m_end0"]["at"], insts["m_end0"]["face"], insts["m_end0"]["surface"]),
                         ("pipe_flange", (32400, 29200), 270.0, "m_l0.line"))
        self.assertEqual((insts["m_end1"]["at"], insts["m_end1"]["face"], insts["m_end1"]["surface"]),
                         ((50400, 23200), 90.0, "m_l4.line"))

    def test_a_cap_where_the_pipe_says(self):
        insts = by_key(plan(pts=MAIN[:2], heights=[4.5, 4.5], ends=["cap", "flange"]), "instances")
        self.assertEqual((insts["m_end0"]["asset"], insts["m_end1"]["asset"]), ("pipe_cap", "pipe_flange"))

    def test_ends_is_two_words_of_the_two(self):
        for bad in (["flange"], ["flange", "plug"], "cap", ["cap", "cap", "cap"]):
            self.assertEqual(error(self, pts=MAIN[:2], heights=[4.5, 4.5], ends=bad).key, "ends")

    def test_an_end_the_kit_has_no_piece_for_is_refused(self):
        e = error(self, pts=MAIN[:2], heights=[4.5, 4.5], ends=["flange", "cap"], k=kit(drop=("pipe_cap",)))
        self.assertEqual(e.message, "`pipe` has no `role:cap` piece, which the pipe's last end takes")
        e = error(self, pts=MAIN[:2], heights=[4.5, 4.5], k=kit(drop=("pipe_flange",)))
        self.assertIn("`role:flange` piece, which the pipe's first end takes", e.message)


class Fit(unittest.TestCase):
    """Rule 5: straight pieces and slip pieces between the pieces at a leg's ends."""

    def test_the_most_straight_pieces_and_the_rest_in_slip_pieces(self):
        runs = by_key(plan(), "runs")
        # leg 0: 4 m, 0.5 for the flange and 0.5 for the riser: one straight piece and two slips
        self.assertEqual((runs["m_l0_s"]["asset"], runs["m_l0_s"]["count"], runs["m_l0_s"]["ref"], runs["m_l0_s"]["span"]),
                         ("pipe", 1, "m_l0", [0.125, 0.625]))
        self.assertEqual((runs["m_l0_f"]["asset"], runs["m_l0_f"]["count"], runs["m_l0_f"]["span"]), ("pipe_slip", 2, [0.625, 0.875]))
        # leg 2: 2 m, 1 m of slips and no straight piece
        self.assertNotIn("m_l2_s", runs)
        self.assertEqual((runs["m_l2_f"]["count"], runs["m_l2_f"]["span"]), (2, [0.25, 0.75]))

    def test_a_straight_piece_lies_along_its_leg(self):
        runs = by_key(plan(), "runs")
        self.assertEqual((runs["m_l0_s"]["face"], runs["m_l3_s"]["face"]), (0.0, 90.0))       # east: its x east; south: its x south

    def test_a_leg_that_is_no_whole_number_of_pieces_is_open_with_what_closes(self):
        out = plan(pts=[(0, 0), (4200, 0)], heights=[4.5, 4.5])
        self.assertEqual(out["runs"], [])
        (n,) = out["notes"]
        self.assertEqual((n["code"], n["leg"], n["length"], n["takes"], n["near"], n["at"]),
                         ("pipe_open", 0, 4200.0, 1000, [4000, 4500], (2100.0, 0.0)))
        self.assertEqual(sorted(i["key"] for i in out["instances"]), ["m_end0", "m_end1"])       # the end pieces stand

    def test_a_leg_shorter_than_its_ends_take_is_open_with_the_least_it_may_be(self):
        (n,) = plan(pts=[(0, 0), (800, 0)], heights=[4.5, 4.5])["notes"]
        self.assertEqual((n["code"], n["takes"], n["near"]), ("pipe_open", 1000, [1000]))

    def test_a_leg_that_is_only_its_end_pieces_holds_no_run_and_is_not_open(self):
        out = plan(pts=[(0, 0), (1000, 0)], heights=[4.5, 4.5])
        self.assertEqual((out["runs"], [n for n in out["notes"] if n["code"] == "pipe_open"]), ([], []))

    def test_a_kit_with_no_slip_closes_on_whole_straight_pieces_only(self):
        k = kit(drop=("pipe_slip",))
        self.assertEqual([r["key"] for r in plan(pts=[(0, 0), (5000, 0)], heights=[4.5, 4.5], k=k)["runs"]], ["m_l0_s"])
        (n,) = plan(pts=[(0, 0), (4500, 0)], heights=[4.5, 4.5], k=k)["notes"]
        self.assertEqual(n["near"], [3000, 5000])

    def test_a_leg_closes_five_millimetres_long_and_one_and_a_half_short(self):
        fit = P["fit"]
        self.assertEqual([fit(4000 + d, 2000, 500)[0] for d in (-2, -1.5, 0, 5, 6)], [None, (2, 0), (2, 0), (2, 0), None])
        self.assertEqual(fit(4200, 2000, 500), (None, (4000, 4500)))
        self.assertEqual(fit(300, 2000, 0), (None, (0, 2000)))

    def test_a_turn_is_right_when_positive(self):
        self.assertEqual([P["turn"](a, b) for a, b in ((90, 180), (180, 90), (350, 10), (10, 350), (0, 180))],
                         [90, -90, 20, -20, 180])
        self.assertEqual([P["bearing"](v) for v in ((0, 1), (1, 0), (0, -1), (-1, 0))], [0.0, 90.0, 180.0, 270.0])


class Supports(unittest.TestCase):
    """Rule 6: supports under a leg at the height the kit's support is made for."""

    def test_as_many_as_keep_them_within_the_kits_spacing_evenly_along_the_pieces(self):
        out = plan()
        sup = {k: v for k, v in by_key(out, "instances").items() if "_p" in k}
        # leg 0: 3 m of pieces from 0.5 m, one support at its middle; leg 3: 5 m, two
        self.assertEqual({k: v["at"] for k, v in sup.items()},
                         {"m_l0_p0": (34400, 29200), "m_l2_p0": (47400, 29200), "m_l3_p0": (48400, 27450),
                          "m_l3_p1": (48400, 24950), "m_l4_p0": (49400, 23200)})
        self.assertEqual(out["supports"][0], ("m_l0_p0", (34400, 29200)))

    def test_a_support_is_a_piece_of_its_leg_on_the_legs_surface(self):
        s = by_key(plan(), "instances")["m_l3_p0"]
        self.assertEqual((s["asset"], s["surface"], s["face"]), ("pipe_support", "m_l3.line", 90.0))

    def test_a_leg_at_another_height_has_none_and_the_pipe_says_how_many(self):
        out = plan()
        self.assertFalse([k for k in by_key(out, "instances") if k.startswith("m_l1_p")])
        (n,) = [n for n in out["notes"] if n["code"] == "support_none"]
        self.assertEqual((n["legs"], n["height"]), (1, 4500))

    def test_a_kit_with_no_spacing_puts_one_under_each_leg(self):
        out = plan(k=kit(shared={"section": 0.3}))
        self.assertEqual(sorted(k for k in by_key(out, "instances") if k.startswith("m_l3_p")), ["m_l3_p0"])

    def test_a_kit_with_no_support_says_nothing_of_it(self):
        out = plan(k=kit(drop=("pipe_support",)))
        self.assertEqual((out["supports"], out["notes"]), ([], []))

    def test_a_leg_that_is_only_its_end_pieces_has_no_support(self):
        out = plan(pts=[(0, 0), (1000, 0)], heights=[4.5, 4.5])
        self.assertEqual(out["supports"], [])


class FakeOut:
    def __init__(self):
        self.features, self.instances, self.tables = {}, {}, {}

    def feature(self, key, kind, at=None, path=None, region=None, **attrs):
        self.features[key] = dict(attrs, kind=kind, path=path)

    def instance(self, key, asset, at, face=None, **keys):
        self.instances[key] = dict(keys, asset=asset, at=at, face=face)

    def table(self, section, key, **keys):
        self.tables[(section, key)] = keys


class _FL(dict):
    def rings(self, f):
        return [[[self["coords"][f]]]]


class FakeStep:
    """The step as `pipes` reads it: pipes by name with their attributes, the pipe kit's rows,
    and ground that is 10 m but where `ground` says."""

    def __init__(self, feats, rows=None, ground=None, item=None, extent=None):
        self.grid, self.id, self.out, self.said, self.for_each_item = None, "pipes", FakeOut(), [], item
        self.feats, self.rows, self.ground, self.at = feats, pipe_rows() if rows is None else rows, ground or {}, []
        if extent:                              # mm: [x0, y0, x1, y1], as the engine gives a step its grid
            self.grid = {"extent_mm": list(extent)}

    def features(self, select, columns=()):
        names = select if isinstance(select, list) else sorted(self.feats)
        fl = _FL({"name": names, "coords": [list(self.feats[n][0]) for n in names]})
        for c in columns:
            fl[c], fl[c + "?"] = [], []
            for n in names:
                a = self.feats[n][1]
                v = a.get(c)
                fl[c].append(json.dumps(v) if isinstance(v, (list, dict)) else (v if c in a else 0))
                fl[c + "?"].append(1 if c in a else 0)
        return fl

    def fields(self, names, **kw):
        class A(list):
            name = "height"
        return [A()]

    def eval(self, formula, x, y):
        # The engine refuses a point outside the extent (protocol.md 3.4), as the real step saw.
        if self.grid:
            x0, y0, x1, y1 = self.grid["extent_mm"]
            for px, py in zip(x, y):
                if not (x0 <= px <= x1 and y0 <= py <= y1):
                    raise P["RpcError"](-32602, f"({px}, {py}) is outside the extent")
        return [self.ground.get((px, py), 10.0) for px, py in zip(x, y)]

    def pack(self, select, columns=()):
        rows = self.rows if select == "@service/pipe" else []
        return {"id": [r["id"] for r in rows], "tags": [json.dumps(r["tags"]) for r in rows],
                "bounds": [r["bounds"] for r in rows], "params": [json.dumps(r["params"]) for r in rows],
                "shared": [json.dumps(r["shared"]) for r in rows]}

    def _say(sev):
        def say(self, code, subject, message, hint, **kw):
            # As the engine holds a step's record (`diag.malformed`): each `data` value is a
            # string, a number, a boolean or a list of those, and a position is two finite numbers.
            for k, v in (kw.get("data") or {}).items():
                for x in (v if isinstance(v, list) else [v]):
                    if not isinstance(x, (str, int, float, bool)) or (isinstance(x, float) and not math.isfinite(x)):
                        raise AssertionError(f"`data.{k}` is not a string, number, boolean or list of those: {v!r}")
            pos = kw.get("position")
            if pos is not None and not (len(pos) == 2 and all(math.isfinite(c) for c in pos)):
                raise AssertionError(f"`position` is {pos!r}")
            self.said.append((sev, code, subject, message, hint, kw.get("data")))
            self.at.append(pos)
        return say
    error, warning, info = _say("error"), _say("warning"), _say("info")


TO = [[36.4, 29.2], [46.4, 29.2], [48.4, 29.2], [48.4, 23.2], [50.4, 23.2]]
WORLD = {"main": (MAIN[0], {"to": TO, "heights": HEIGHTS, "kit": "service/pipe"})}


def run(feats=None, **kw):
    s = FakeStep(WORLD if feats is None else feats, **kw)
    P["generate"](s)
    return s


class Step(unittest.TestCase):
    """pipe.py's step, through a fake of the SDK."""

    def test_legs_runs_and_pieces_are_written_as_the_engine_takes_them(self):
        s = run()
        f = s.out.features
        self.assertEqual(sorted(f), ["main_l0", "main_l1", "main_l2", "main_l3", "main_l4"])
        self.assertEqual(f["main_l0"], {"kind": "pipe_leg", "path": [(32400, 29200), (36400, 29200)], "level": 14.5, "width": 0.3})
        self.assertEqual(f["main_l1"], {"kind": "pipe_leg", "path": [], "level": 17.5, "width": 0.3,
                                        "starts_at": "main_r1.top", "ends_at": "main_r2.top"})
        self.assertEqual(s.out.tables[("run", "main_l0_s")],
                         {"asset": "pipe", "count": 1, "ends": "half", "face": 0.0,
                          "on": {"ref": "main_l0", "span": [0.125, 0.625], "offset": 0}})
        self.assertEqual(s.out.instances["main_end0"], {"asset": "pipe_flange", "at": (32400, 29200), "face": 270.0,
                                                        "surface": "main_l0.line"})
        self.assertEqual(s.out.instances["main_l0_p0"]["surface"], "main_l0.line")

    def test_it_says_what_it_returned(self):
        s = run()
        self.assertEqual([(x[0], x[1], x[2]) for x in s.said], [("info", "support_none", "main"), ("info", "pipes", "step.pipes")])
        self.assertEqual(s.said[1][3], "1 pipes, 5 legs, 28 pieces")
        self.assertEqual(s.said[1][5], {"pipes": 1, "legs": 5, "pieces": 28})
        self.assertEqual(s.said[0][3], "1 of `main`'s legs have no support: the kit's stands under a pipe 4.5 m up")

    def test_the_pipes_heights_are_over_the_ground_at_its_first_point(self):
        s = run(ground={MAIN[0]: 12.25})
        self.assertEqual([s.out.features[k]["level"] for k in ("main_l0", "main_l1")], [16.75, 19.75])

    def test_a_fault_is_said_of_its_pipe_and_the_others_are_laid(self):
        feats = dict(WORLD, spur=((29200, 38400), {"to": [[29.2, 44.4]], "heights": [4.5, 5], "kit": "service/pipe"}))
        s = run(feats)
        (e,) = [x for x in s.said if x[0] == "error"]
        # the record's figures are its data: the leg and the two heights (a stopped build is told by them)
        self.assertEqual((e[1], e[2], e[5]), ("params", "spur", {"attribute": "heights", "leg": 0, "from": 4.5, "to": 5.0}))
        self.assertTrue(e[3].startswith("`spur`: leg 0, from point 0 to point 1, climbs from 4.5 m to 5 m"))
        self.assertIn("main_l0", s.out.features)
        self.assertFalse([k for k in s.out.features if k.startswith("spur")])

    def test_a_pipe_with_no_kit_and_a_kit_that_is_not_in_the_pack(self):
        s = run({"a": (MAIN[0], {"to": TO, "heights": HEIGHTS}),
                 "b": (MAIN[0], {"to": TO, "heights": HEIGHTS, "kit": "service/none"})})
        self.assertEqual([x[3] for x in s.said if x[0] == "error"],
                         ["`a`: the pipe names no kit", "`b`: no piece of `service/none` is in the pack"])

    def test_a_pipe_with_no_points_to_go_through_is_refused(self):
        s = run({"a": (MAIN[0], {"heights": [4.5], "kit": "service/pipe"})})
        self.assertEqual([(x[1], x[5]) for x in s.said if x[0] == "error"], [("params", {"attribute": "to"})])

    def test_an_open_leg_is_said_with_the_lengths_that_close(self):
        s = run({"a": ((0, 0), {"to": [[4.2, 0]], "heights": [4.5, 4.5], "kit": "service/pipe"})})
        (w,) = [x for x in s.said if x[1] == "pipe_open"]
        self.assertEqual(w[3], "leg 0 of `a` is 4.2 m and the pieces at its ends take 1 m: what is left is no whole number "
                               "of the kit's pieces, and none is laid")
        self.assertEqual(w[4], "make the leg 4 m or 4.5 m long")
        self.assertEqual(w[5], {"leg": 0, "length": 4.2, "takes": 1.0, "near": [4.0, 4.5]})

    def test_a_leg_shorter_than_its_ends_is_told_the_least_it_may_be(self):
        s = run({"a": ((0, 0), {"to": [[0.8, 0]], "heights": [4.5, 4.5], "kit": "service/pipe"})})
        (w,) = [x for x in s.said if x[1] == "pipe_open"]
        self.assertEqual((w[4], w[5]["near"]), ("make the leg 1 m long", [1.0]))

    def test_ground_that_is_not_the_first_points_under_a_support_is_said(self):
        s = run(ground={(48400, 27450): 10.4, (48400, 24950): 9.9, (49400, 23200): 10.04})
        (w,) = [x for x in s.said if x[1] == "support_off"]
        self.assertEqual(w[2], "main")
        self.assertIn("the ground under 2 of `main`'s supports is not the ground under its first point: under `main_l3_p0` "
                      "it is 0.40 m higher", w[3])
        self.assertEqual(w[5], {"supports": 2, "most": 0.4})
        s = run(ground={(48400, 27450): 9.5})
        self.assertIn("0.50 m lower", [x for x in s.said if x[1] == "support_off"][0][3])
        self.assertFalse([x for x in run(ground={(48400, 27450): 10.05}).said if x[1] == "support_off"])

    def test_called_for_one_pipe_it_lays_that_pipe_and_says_so_of_it(self):
        feats = dict(WORLD, spur=((29200, 38400), {"to": [[29.2, 44.4]], "heights": [4.5, 4.5], "kit": "service/pipe",
                                                   "ends": ["flange", "cap"]}))
        s = run(feats, item="spur")
        self.assertEqual(sorted(s.out.features), ["spur_l0"])
        self.assertEqual(s.out.instances["spur_end1"]["asset"], "pipe_cap")
        self.assertEqual([(x[0], x[1], x[2], x[3]) for x in s.said], [("info", "pipes", "spur", "1 legs, 8 pieces")])

    def test_called_for_a_pipe_that_is_refused_it_says_the_error_alone(self):
        feats = dict(WORLD, spur=((29200, 38400), {"to": [[29.2, 44.4]], "heights": [4.5, 5], "kit": "service/pipe"}))
        s = run(feats, item="spur")
        self.assertEqual([(x[0], x[1], x[2]) for x in s.said], [("error", "params", "spur")])
        self.assertEqual(s.out.features, {})

    def test_a_world_with_no_pipe_returns_nothing(self):
        s = run({})
        self.assertEqual((s.out.features, [x[3] for x in s.said]), ({}, ["0 pipes, 0 legs, 0 pieces"]))


# ---- lay.py rule 8: a port is a variant of the bay, and stays in its run

def wrow(id_, roles, size, high, tags=(), shared=None, mounts=None, **params):
    w, t = size
    roles = [roles] if isinstance(roles, str) else roles
    return {"id": id_, "tags": [f"role:{r}" for r in roles] + list(tags),
            "bounds": [-w // 2, -t // 2, 0, w - w // 2, t - t // 2, high], "params": dict(params),
            "shared": shared or {}, "mounts": mounts or {}}


PORT = {"port": {"at": [0, 0.2], "face": 0, "rise": 4.5, "connector": "pipe"}}
SHED = {"wall": 0.2, "storey": 7.2, "door": 3.0}
BOILER = {"wall": 0.3, "storey": 6.0, "door": 1.5}


def shed_rows(port=PORT):
    return [wrow("shed_post", ["post", "corner"], (400, 400), 7200, shared=SHED),
            wrow("shed_panel", "wall", (5600, 200), 7200, shared=SHED, pitch=6.0),
            wrow("shed_port", "wall", (5600, 200), 7200, ("variant:port",), SHED, port, pitch=6.0),
            wrow("shed_door", "door", (5600, 200), 7200, shared=SHED, pitch=6.0)]


def boiler_rows(port=PORT):
    return [wrow("boiler_wall", "wall", (3000, 300), 6000, shared=BOILER, pitch=3.0),
            wrow("boiler_port", "wall", (3000, 300), 6000, ("variant:port",), BOILER, port, pitch=3.0),
            wrow("boiler_corner", "corner", (300, 300), 6000, shared=BOILER),
            wrow("boiler_door", "door", (3000, 300), 6000, shared=BOILER, pitch=3.0)]


def lay(ring, rows, storey_h, **attrs):
    a = {"storeys": 1, "storey_h": storey_h, "plinth": 0, "porch": 2, "walk_out": 3, "kit": "k", **attrs}
    return L["plan"]("b", ring, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h),
                     foot=[(p[0], p[1], 10.0) for p in ring])


SHED_RING = [(8000, 20000), (32400, 20000), (32400, 38400), (8000, 38400)]          # four bays by three, and a post
BOILER_RING = [(50400, 18400), (60000, 18400), (60000, 25000), (50400, 25000)]      # three bays by two, and two corners


class Joined(unittest.TestCase):
    """A variant that carries a mount (a port) is laid as any variant is: a member of its
    stretch's run, named by `assets`, so a bay keeps its name whichever piece it holds (rule 8,
    R68). The engine finds its joint and says what it must of a run's member (K9's fix pass)."""

    def test_a_port_between_posts_stays_in_its_run(self):
        out = lay(SHED_RING, shed_rows(), 7.2, bays=[{"storey": 0, "side": 1, "at": 9.2, "variant": "port"}])
        (r,) = [r for k, r in by_key(out, "runs").items() if k.startswith("b_s0_w1_") and r.get("assets")]
        self.assertEqual((r["key"], r["count"], r["assets"]), ("b_s0_w1_0", 3, ["shed_panel", "shed_port", "shed_panel"]))
        self.assertFalse([k for k in by_key(out, "instances") if k.startswith("b_s0_w1_")])

    def test_a_port_in_a_wall_at_a_pitch_stays_in_its_run(self):
        out = lay(BOILER_RING, boiler_rows(), 6.0, bays=[{"storey": 0, "side": 3, "at": 1.8, "variant": "port"}])
        (r,) = [r for k, r in by_key(out, "runs").items() if k.startswith("b_s0_w3_")]
        self.assertEqual((r["key"], r["assets"]), ("b_s0_w3_0", ["boiler_port", "boiler_wall"]))



if __name__ == "__main__":
    unittest.main()


# ---- K9's fix pass: what the review found in pipe.py, and its surviving breakages

class Reviewed(unittest.TestCase):
    """Each of these failed, or passed for the wrong reason, before K9's fix pass."""

    def test_a_leg_shorter_than_its_ends_says_no_null(self):
        # 0.6 m, cap to cap: the ends take 1 m. `data.near` held a null and the engine refused the
        # record (`diag.malformed`); the fake step now refuses what the engine does.
        s = run({"p": ((0, 0), {"to": [[0.6, 0]], "heights": [4.5, 4.5], "kit": "service/pipe", "ends": ["cap", "cap"]})})
        (w,) = [x for x in s.said if x[1] == "pipe_open"]
        self.assertEqual((w[4], w[5]["near"]), ("make the leg 1 m long", [1.0]))

    def test_the_fake_step_refuses_a_null_in_data(self):
        s = FakeStep({})
        with self.assertRaises(AssertionError):
            s.warning("x", "p", "m", "h", data={"near": [None, 1.0]})

    def test_a_pipe_that_leaves_the_extent_is_refused_by_its_point(self):
        feats = {"p": ((60000, 60000), {"to": [[70, 60]], "heights": [4.5, 4.5], "kit": "service/pipe"})}
        s = run(feats, extent=(0, 0, 64000, 64000))
        (e,) = [x for x in s.said if x[0] == "error"]
        self.assertEqual((e[1], e[3]), ("params", "`p`: point 1 of the pipe, [70, 60], is outside the world's extent"))
        self.assertEqual(s.out.features, {})

    def test_a_pipe_on_the_extents_edge_is_laid(self):
        feats = {"p": ((60000, 60000), {"to": [[64, 60]], "heights": [4.5, 4.5], "kit": "service/pipe"})}
        s = run(feats, extent=(0, 0, 64000, 64000))
        self.assertEqual([x for x in s.said if x[0] == "error"], [])
        self.assertIn("p_l0", s.out.features)

    def test_a_support_that_says_no_height_is_refused(self):
        with self.assertRaises(PE) as c:
            kit(swap={"pipe_support": prow("pipe_support", "support")})
        self.assertEqual(c.exception.message, "the support `pipe_support` says no `params.height`")

    def test_a_climb_a_millimetre_off_the_riser_is_refused(self):
        # It passed here and was `surface.gap`, 1 mm, in the engine.
        e = error(self, heights=[4.5, [4.5, 7.501], [7.501, 4.5], 4.5, 4.5, 4.5])
        self.assertEqual(e.message, "the pipe climbs 3.001 m at point 1: `pipe` has a riser of 3 m")

    def test_a_leg_three_millimetres_off_the_supports_height_has_no_support(self):
        # It was laid with supports 3 mm into the ground, and nothing said.
        out = plan(pts=MAIN[:2], heights=[4.497, 4.497])
        self.assertEqual(out["supports"], [])
        self.assertEqual([(n["code"], n["legs"], n["at"]) for n in out["notes"]], [("support_none", 1, MAIN[0])])

    def test_the_hint_for_a_leg_that_climbs_is_accepted_when_followed(self):
        # leg 0 of three legs in one line climbs: the riser goes at point 1, and writing it builds
        pts = [(0, 0), (4000, 0), (8000, 0), (12000, 0)]
        e = error(self, pts=pts, heights=[4.5, 7.5, 7.5, 7.5])
        self.assertIn("`heights[1] = [4.5, 7.5]` is a riser there", e.hint)
        self.assertNotIn("The kit's riser makes", e.hint)
        out = plan(pts=pts, heights=[4.5, [4.5, 7.5], 7.5, 7.5])
        self.assertIn("m_r1", by_key(out, "instances"))
        # the last leg climbs: its far point is the pipe's last, so the riser goes at its near one
        e = error(self, pts=pts, heights=[4.5, 4.5, 4.5, 7.5])
        self.assertIn("`heights[2] = [4.5, 7.5]` is a riser there", e.hint)
        self.assertIn("m_r2", by_key(plan(pts=pts, heights=[4.5, 4.5, [4.5, 7.5], 7.5]), "instances"))

    def test_a_pipe_of_one_leg_that_climbs_is_told_a_point_is_wanted(self):
        e = error(self, pts=MAIN[:2], heights=[4.5, 7.5])
        self.assertTrue(e.hint.startswith("give both ends one height. A riser stands at a point between two legs in one line"))

    def test_the_hint_does_not_put_a_riser_where_the_pipe_turns(self):
        pts = [(0, 0), (4000, 0), (4000, 4000), (8000, 4000)]      # a turn at both ends of leg 1
        e = error(self, pts=pts, heights=[4.5, 4.5, 7.5, 7.5])
        self.assertNotIn("is a riser there", e.hint)

    def test_an_elbow_of_45_degrees_is_laid_to_either_hand(self):
        k = kit(swap={"pipe_elbow": prow("pipe_elbow", "elbow", angle=45, reach=0.5)})
        north = [(0, 0), (0, 4000)]
        right = by_key(plan(pts=north + [(2000, 6000)], heights=[4.5, 4.5, 4.5], k=k), "instances")["m_e1"]
        left = by_key(plan(pts=north + [(-2000, 6000)], heights=[4.5, 4.5, 4.5], k=k), "instances")["m_e1"]
        # the piece's arms: one to its back (face + 180), one 45 degrees right of its front (face + 45)
        self.assertEqual(right["face"], 0.0)            # back arm south, where the pipe came from; other arm north-east
        self.assertEqual(left["face"], 135.0)           # back arm north-west, where it goes; other arm south (135 + 45)
        for face, arrive, leave in ((right["face"], 0.0, 45.0), (left["face"], 0.0, 315.0)):
            arms = {round((face + 180.0) % 360.0, 3), round((face + 45.0) % 360.0, 3)}
            self.assertEqual(arms, {round((arrive + 180.0) % 360.0, 3), leave})

    def test_the_nearest_length_above_may_need_one_more_straight_piece(self):
        self.assertEqual(P["fit"](3900, 2000, 0), (None, (2000, 4000)))

    def test_the_last_ends_piece_stands_on_the_last_leg(self):
        insts = by_key(plan(), "instances")
        self.assertEqual((insts["m_end0"]["surface"], insts["m_end1"]["surface"]), ("m_l0.line", "m_l4.line"))

    def test_a_leg_a_millimetre_shorter_than_its_ends_closes(self):
        out = plan(pts=[(0, 0), (999, 0)], heights=[4.5, 4.5], ends=["cap", "cap"])
        self.assertEqual([n for n in out["notes"] if n["code"] == "pipe_open"], [])
        out = plan(pts=[(0, 0), (998, 0)], heights=[4.5, 4.5], ends=["cap", "cap"])
        self.assertEqual([n["near"] for n in out["notes"] if n["code"] == "pipe_open"], [[1000]])

    def test_what_is_said_of_a_pipe_inside_the_extent_has_its_place(self):
        feats = {"p": ((10000, 10000), {"to": [[13.3, 10]], "heights": [4.5, 4.5], "kit": "service/pipe"})}
        s = run(feats, extent=(0, 0, 64000, 64000))
        i = [x[1] for x in s.said].index("pipe_open")
        self.assertEqual(s.at[i], (11.65, 10.0))

    def test_a_turn_and_a_climb_that_are_refused_say_their_figures(self):
        e = error(self, pts=[(0, 0), (4000, 0), (8000, 4000)], heights=[4.5, 4.5, 4.5])
        self.assertEqual(e.data, {"point": 1, "turn": 45.0})
        e = error(self, pts=MAIN[:3], heights=[4.5, [4.5, 6.5], 6.5])
        self.assertEqual(e.data, {"point": 1, "climb": 2.0})

    def test_a_kit_that_states_no_section_gives_its_legs_0_3_m(self):
        s = run(rows=pipe_rows(shared={"connector": "pipe", "support": 4.0}))
        self.assertEqual(s.out.features["main_l0"]["width"], 0.3)
