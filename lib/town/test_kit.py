"""Tests for kit.py, the kit run step: ends, corners, fillers, spacing and steps.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/town -p 'test_*.py'

The kit is lib/packs/starter's stone wall: a 2 m bay, a 0.5 m filler, a 0.6 m end pier and a
0.7 m corner pier, each sunk 0.5 m.
"""

import math
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
K = runpy.run_path(os.path.join(HERE, "kit.py"))
T = runpy.run_path(os.path.join(HERE, "test_streets.py"))

ROWS = [
    {"id": "wall_bay", "tags": ["role:wall"], "bounds": [-1000, -250, 0, 1000, 250, 1600],
     "params": {"pitch": 2.0}, "sink": 0.5},
    {"id": "wall_filler", "tags": ["role:filler"], "bounds": [-250, -250, 0, 250, 250, 1600],
     "params": {"pitch": 0.5}, "sink": 0.5},
    {"id": "wall_pier", "tags": ["role:end"], "bounds": [-300, -350, 0, 300, 350, 1900],
     "params": {"pitch": 0.6}, "sink": 0.5},
    {"id": "wall_corner", "tags": ["role:corner"], "bounds": [-350, -400, 0, 350, 400, 2000],
     "params": {"pitch": 0.7}, "sink": 0.5},
]
KIT = K["kit_pieces"](ROWS)


def flat(_s):
    return 0.0


class KitTest(unittest.TestCase):
    def test_pieces_by_role(self):
        self.assertEqual(sorted(KIT), ["corner", "end", "filler", "wall"])
        self.assertEqual((KIT["wall"]["pitch"], KIT["filler"]["sink"]), (2.0, 0.5))
        self.assertAlmostEqual(KIT["corner"]["depth"], 0.8)

    def test_the_bay_is_never_a_variant(self):
        # A variant sorts before the bay by id; the bay is still the piece with no `variant:` tag
        # (docs/design/kit-contract.md). With variants alone, the first id stands.
        arch = {"id": "arch_bay", "tags": ["role:wall", "variant:arch"], "bounds": [-1000, -250, 0, 1000, 250, 1600],
                "params": {"pitch": 2.0}, "sink": 0.5}
        self.assertEqual(K["kit_pieces"]([arch] + ROWS)["wall"]["id"], "wall_bay")
        self.assertEqual(K["kit_pieces"]([arch])["wall"]["id"], "arch_bay")

    def test_ends_inclusive(self):
        # 10.6 m with an end pier on each end: the piers are centred on the stretch's two ends,
        # and between their faces (9.4 m... less 0.3 each side: 10.0 m) four bays and four fillers.
        got, closes = K["lay_stretch"](10.6, KIT, "end", "end")
        self.assertIsNone(closes)
        self.assertEqual((got[0][:2], got[-1][:2]), (("end", 0.0), ("end", 10.6)))
        roles = [r for r, _, _ in got]
        self.assertEqual(roles.count("wall"), 5)
        self.assertEqual(roles.count("filler"), 0)

    def test_fillers_split_round_the_bays(self):
        # 0.3 + 9.0 + 0.3: four bays and two fillers, one at each end of the bays.
        got, _ = K["lay_stretch"](9.6, KIT, "end", "end")
        roles = [r for r, _, _ in got]
        self.assertEqual(roles, ["end", "filler", "wall", "wall", "wall", "wall", "filler", "end"])
        # Touching, end to end: each piece starts where the last one ended.
        for (_, m0, l0), (_, m1, l1) in zip(got[1:-1], got[2:-1]):
            self.assertAlmostEqual(m0 + l0 / 2, m1 - l1 / 2, places=9)
        self.assertAlmostEqual(got[1][1] - got[1][2] / 2, 0.3, places=9)

    def test_spacing_spreads_what_is_left_over_the_joints(self):
        # 0.3 + 9.2 + 0.3: what whole pieces leave (0.2 m) goes to the 7 joints, 28.6 mm each.
        got, closes = K["lay_stretch"](9.8, KIT, "end", "end")
        self.assertIsNone(closes)
        inner = got[1:-1]
        gaps = [inner[0][1] - inner[0][2] / 2 - 0.3]
        gaps += [b[1] - b[2] / 2 - (a[1] + a[2] / 2) for a, b in zip(inner, inner[1:])]
        gaps += [9.8 - 0.3 - (inner[-1][1] + inner[-1][2] / 2)]
        self.assertEqual(len(gaps), len(inner) + 1)
        for g in gaps:
            self.assertAlmostEqual(g, 0.2 / 7, places=9)

    def test_open_names_the_lengths_that_close(self):
        # With no filler and no joint allowed, 9.8 m does not close: 8.6 and 10.6 m would.
        kit = {k: v for k, v in KIT.items() if k != "filler"}
        got, closes = K["lay_stretch"](9.8, kit, "end", "end", joint=0.0)
        self.assertIsNone(got)
        self.assertEqual(closes, (8.6, 10.6))

    def test_corner_reach_keeps_bays_off_the_pier(self):
        # A corner pier 0.7 by 0.8 m: bays begin half its diagonal (0.532 m) from the vertex, so a
        # 0.5 m thick bay cannot meet the pier's footprint at any turn of the path.
        reach = K["cap_reach"](KIT, "corner")
        self.assertAlmostEqual(reach, math.hypot(0.7, 0.8) / 2)
        for turn in (10, 45, 90, 135):
            a = math.radians(turn)
            t = (math.cos(a), math.sin(a))             # the next leg leaves the vertex along t
            n = (-t[1], t[0])
            bay = [(t[0] * s + n[0] * o, t[1] * s + n[1] * o)
                   for s in (reach, reach + 2.0) for o in (-0.25, 0.25)]
            pier = [(x, y) for x in (-0.35, 0.35) for y in (-0.4, 0.4)]
            # Separated along t: every bay corner lies further along t than any pier corner.
            self.assertGreater(min(p[0] * t[0] + p[1] * t[1] for p in bay),
                               max(p[0] * t[0] + p[1] * t[1] for p in pier) - 1e-9)

    def test_short_stretches(self):
        # Shorter than its caps: one end pier at its middle when one fits (beside a corner, whose
        # reach is 0.53 m), else nothing; between its caps and caps plus a filler: open.
        self.assertEqual(K["lay_stretch"](0.4, KIT, "end", "end")[0], [])
        self.assertEqual(K["lay_stretch"](0.7, KIT, "corner", "end")[0], [("end", 0.35, 0.6)])
        self.assertEqual(K["lay_stretch"](0.9, KIT, "end", "end"), (None, (0.6, 1.1)))

    def test_legs_cut_at_corners_only(self):
        square = [(0, 0), (10, 0), (10, 10), (0, 10)]
        cut, corners = K["legs"](square)
        self.assertEqual([i for i, _, _ in corners], [1, 2])
        self.assertEqual(len(cut), 3)
        bent = [(0, 0), (10, 0), (20, 0.8)]            # a 4.6 degree bend: walked through
        cut, corners = K["legs"](bent)
        self.assertEqual((len(cut), corners), (1, []))

    def test_gaps_cut_stretches_with_end_caps(self):
        st = K["stretches"](30.0, [(10.0, 14.0)], "corner", "end")
        self.assertEqual(st, [(0.0, 10.0, "corner", "gap"), (14.0, 30.0, "gap", "end")])
        st = K["stretches"](30.0, [(-2.0, 3.0)], "corner", "corner")
        self.assertEqual(st, [(3.0, 30.0, "gap", "corner")])

    def test_a_cap_at_a_gap_slides_into_it(self):
        # quay move_hill: 3.75 m between a corner (reach 0.53 m) and a gap's edge. One bay and one
        # filler leave 0.42 m, 0.14 m a joint: open between fixed caps; at a gap the end pier
        # moves 0.42 m into the gap instead and every piece touches.
        self.assertIsNone(K["lay_stretch"](3.75, KIT, "corner", "end")[0])
        got, closes = K["lay_stretch"](3.75, KIT, "corner", "gap")
        self.assertIsNone(closes)
        reach = K["cap_reach"](KIT, "corner")
        inner = got[:-1]
        self.assertAlmostEqual(inner[0][1] - inner[0][2] / 2, reach, places=9)
        for a, b in zip(inner, inner[1:]):
            self.assertAlmostEqual(a[1] + a[2] / 2, b[1] - b[2] / 2, places=9)
        self.assertAlmostEqual(got[-1][1], reach + 2.5 + 0.3, places=9)
        self.assertLess(got[-1][1], 3.75)

    def test_gap_spans_widen_by_a_post(self):
        r = [0, 0, 0, 1, 1, 0, 0, 0]                   # readings every 0.25 m: 0.75 to 1.0 m
        self.assertEqual(K["gap_spans"](r), [(0.25, 1.5)])

    def test_flat_run_is_not_stepped(self):
        got, _ = K["lay_stretch"](9.6, KIT, "end", "end")
        pieces, steep = K["steps"](got, flat, KIT)
        self.assertEqual((len(pieces), steep), (len(got), 0))
        self.assertEqual(K["max_step"](pieces, flat), 0.0)

    def test_steep_bay_laid_as_fillers(self):
        # A 0.3 grade: a 2 m bay's end lies 0.3 m off its origin (within its 0.5 m sink); at 0.6 it
        # lies 0.6 m off, so each bay is laid as four 0.5 m fillers (0.15 m off each).
        got, _ = K["lay_stretch"](9.6, KIT, "end", "end")
        pieces, steep = K["steps"](got, lambda s: 0.3 * s, KIT)
        self.assertEqual([p[0] for p in pieces].count("wall"), 4)
        pieces, steep = K["steps"](got, lambda s: 0.6 * s, KIT)
        self.assertEqual([p[0] for p in pieces].count("wall"), 0)
        self.assertEqual([p[0] for p in pieces].count("filler"), 2 + 16)
        self.assertEqual(steep, 0)
        # Each filler of a cut bay is named from the bay (`i`) and its place in it (`j`).
        self.assertEqual({(p[3], p[4]) for p in pieces if p[4] is not None},
                         {(i, j) for i in (2, 3, 4, 5) for j in range(4)})
        # Stepped: neighbours' tops differ by the ground between their origins.
        self.assertAlmostEqual(K["max_step"](pieces, lambda s: 0.6 * s), 0.6 * 0.55)

    def test_too_steep_even_for_fillers_is_counted(self):
        got, _ = K["lay_stretch"](4.6, KIT, "end", "end")
        _, steep = K["steps"](got, lambda s: 2.5 * s, KIT)
        self.assertGreater(steep, 0)


class KitStepTest(unittest.TestCase):
    def run_step(self, path, keep_out=None, height=None, rows=None, **params):
        rows = rows or ROWS
        fl = {"wall": T["feature_list"]([("wall", 1, path, "feature")])}
        pack = {"id": [r["id"] for r in rows], "tags": [str(r["tags"]).replace("'", '"') for r in rows],
                "bounds": [r["bounds"] for r in rows],
                "params": ['{"pitch": %s}' % r["params"]["pitch"] for r in rows],
                "sink": [r["sink"] for r in rows],             # f64 metres (protocol.md 3.3)
                "footprint": [str(r.get("footprint", [])) for r in rows]}
        step = T["FakeStep"](dict({"runs": ["wall"], "kit": "structure/wall"}, **params),
                             height=height, features=fl, pack=pack)
        step.keep_out = keep_out
        step.id = "kit"
        K["generate"](step)
        return step

    def kit_info(self, step):
        return next(s[3] for s in step.said if s[1] == "kit")

    def test_square_corner_and_a_gap(self):
        # An L of two 20.6 m legs; a road crosses the first leg at x = 10 (keep_out 3 m each side).
        ko = lambda x, y: 1.0 if abs(x - 10) < 3 and y < 2 else 0.0  # noqa: E731
        step = self.run_step([(0, 0), (20.6, 0), (20.6, 20.6)], keep_out=ko)
        info = self.kit_info(step)
        self.assertEqual((info["corner"], info["gaps"], info["open"]), (1, 1, 0))
        names = [i[0] for i in step.out.instances]
        self.assertIn("wall_c1", names)
        # Ends inclusive: an end pier on the path's start and on its end; two more beside the gap.
        ends = [i for i in step.out.instances if i[1] == "wall_pier"]
        self.assertEqual(len(ends), 4)
        self.assertIn((0, 0), [e[2] for e in ends])
        self.assertIn((20600, 20600), [e[2] for e in ends])
        # Nothing in the road: every piece's origin is clear of it.
        for _, _, at, _, _ in step.out.instances:
            self.assertEqual(ko(at[0] / 1000, at[1] / 1000), 0.0)

    def test_fronts_face_the_side(self):
        step = self.run_step([(0, 0), (10.6, 0)], side="right")
        faces = {i[3] for i in step.out.instances}
        self.assertEqual(faces, {180.0})              # east along x, front to the right: south
        step = self.run_step([(0, 0), (10.6, 0)])
        self.assertEqual({i[3] for i in step.out.instances}, {0.0})

    def test_stepped_on_a_slope(self):
        step = self.run_step([(0, 0), (20.6, 0)], height=lambda x, y: 0.6 * x)
        info = self.kit_info(step)
        self.assertGreater(info["split"], 0)
        self.assertEqual(info["steep"], 0)
        self.assertGreater(info["max_step"], 0.0)

    def test_bad_params(self):
        step = self.run_step([(0, 0), (10, 0)], side="up")
        self.assertEqual(step.said[0][:2], ("error", "params"))
        step = self.run_step([(0, 0), (10, 0)], caps="out")
        self.assertEqual(step.said[0][:2], ("error", "params"))

    def test_ends_inside(self):
        # The market's house rows: an end piece stands wholly inside its stretch (its side on the
        # stretch's end), and a free end takes what whole pieces leave, as a gap's edge does.
        # 10.6 m: piers 0.6 each end inside, 9.4 m between: four bays, two fillers, 0.4 m left,
        # half at each free end. Without `inside` the same stretch is open (0.4 m over 7 joints).
        got, closes = K["lay_stretch"](10.6, KIT, "end", "end", inside=True)
        self.assertIsNone(closes)
        self.assertEqual([r for r, _, _ in got],
                         ["end", "filler", "wall", "wall", "wall", "wall", "filler", "end"])
        self.assertAlmostEqual(got[0][1] - got[0][2] / 2, 0.2, places=9)   # inside, slid 0.2
        self.assertAlmostEqual(got[-1][1] + got[-1][2] / 2, 10.4, places=9)
        for (_, m0, l0), (_, m1, l1) in zip(got, got[1:]):                 # wall to wall
            self.assertAlmostEqual(m0 + l0 / 2, m1 - l1 / 2, places=9)
        # A corner keeps its cap fixed: what is left goes to the free end only.
        got, closes = K["lay_stretch"](10.6, KIT, "corner", "end", inside=True)
        self.assertIsNone(closes)
        self.assertAlmostEqual(got[-1][1] + got[-1][2] / 2, 10.6 - (10.6 - 0.6 - math.hypot(0.7, 0.8) / 2) % 0.5, places=9)
        self.assertTrue(all(m - l / 2 >= -1e-9 and m + l / 2 <= 10.6 + 1e-9 for _, m, l in got))
        step = self.run_step([(0, 0), (10.6, 0)], caps="inside")
        self.assertEqual(self.kit_info(step)["open"], 0)
        # A stretch too short for its caps: the one end piece clears the corner's reach.
        got, _ = K["lay_stretch"](1.3, KIT, "corner", "end", inside=True)
        self.assertGreaterEqual(got[0][1] - got[0][2] / 2, math.hypot(0.7, 0.8) / 2 - 1e-9)

    def neighbours_meet(self, step):
        """Pairs of neighbouring pieces (one stretch, k and k + 1) whose plan rectangles, placed
        as the core places them, meet inside."""
        by = {i[0]: i for i in step.out.instances}
        size = {r["id"]: (r["params"]["pitch"] / 2, (r["bounds"][4] - r["bounds"][1]) / 2000) for r in ROWS}
        bad = []
        for name, (_, asset, at, face, _k) in sorted(by.items()):
            head, k = name.rsplit("_", 1)
            nxt = by.get(f"{head}_{int(k) + 1}") if k.isdigit() else None
            if nxt is None:
                continue
            ra = K["_rect"]((at[0] / 1000, at[1] / 1000), face, *size[asset])
            rb = K["_rect"]((nxt[2][0] / 1000, nxt[2][1] / 1000), nxt[3], *size[nxt[1]])
            bad += [(name, nxt[0])] if K["_meet"](ra, rb) else []
        return bad

    def test_gap_reads_the_plan_depth(self):
        # Rule 2: a cross street's verge 2.5 to 3.5 m beside the line: the wall (0.5 m deep) has
        # no gap there; a row of 8 m deep houses (footprint) does, so no house's front reads it.
        ko = lambda x, y: 1.0 if 9 < x < 11 and 2.5 < y < 3.5 else 0.0  # noqa: E731
        step = self.run_step([(0, 0), (40.6, 0)], keep_out=ko)
        self.assertEqual(self.kit_info(step)["gaps"], 0)
        houses = [dict(r, footprint=[[-1000, -4000], [1000, -4000], [1000, 4000], [-1000, 4000]])
                  for r in ROWS]
        step = self.run_step([(0, 0), (40.6, 0)], keep_out=ko, rows=houses)
        self.assertEqual(self.kit_info(step)["gaps"], 1)

    def test_apart_on_the_lattice(self):
        # Rule 7: on a line off the axes, pieces at their exact pitch overlap by rounding to the
        # millimetre; the joint opens a millimetre at a time until they do not.
        step = self.run_step([(0, 0), (20.6 * 0.6, 20.6 * 0.8)], caps="inside")
        self.assertEqual(self.neighbours_meet(step), [])
        self.assertGreater(self.kit_info(step)["opened"], 0)
        # On the axes nothing opens (the quay's wall is laid as before).
        step = self.run_step([(0, 0), (20.6, 0)])
        self.assertEqual(self.kit_info(step)["opened"], 0)

    def test_apart_on_a_bend(self):
        # Rule 7: a 6 degree bend (less than a corner): the joint at the bend opens by about
        # depth * tan(3 degrees), and no neighbours meet; a free end gives up a filler or a bay
        # when the stretch no longer holds them.
        th = math.radians(6)
        path = [(0, 0), (10.3, 0), (10.3 + 10.3 * math.cos(th), 10.3 * math.sin(th))]
        step = self.run_step(path, caps="inside")
        self.assertEqual(self.neighbours_meet(step), [])
        info = self.kit_info(step)
        self.assertGreater(info["opened"], 0)
        self.assertGreaterEqual(info["dropped"], 0)
        got, _ = K["lay_stretch"](20.6, KIT, "end", "end", inside=True)
        cum = K["arc"](path)
        out, opened, dropped = K["apart"](got, KIT, path, cum, 0.0, "left", 20.6)
        self.assertLessEqual(out[-1][1] + out[-1][2] / 2, 20.6 + 1e-6)
        joints = [(b[1] - b[2] / 2) - (a[1] + a[2] / 2) for a, b in zip(out, out[1:])]
        # A bay may straddle the bend, so the turn is taken over two joints: half each at least.
        self.assertGreater(sum(joints), 2 * KIT["wall"]["depth"] * math.tan(th / 4) - 1e-6)


if __name__ == "__main__":
    unittest.main()
