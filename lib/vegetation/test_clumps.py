"""Tests for clumps.py, with a stand-in for the SDK's ``step``.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/vegetation -p 'test_*.py'

The stand-in answers ``eval`` with a constant chance and ``pack`` with fixed ids; formulas
themselves are the engine's (formulas.md), tested there.
"""

import os
import runpy
import unittest

from viasdk import RpcError

C = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), "clumps.py"))
GRID = {"extent_mm": [0, 0, 96000, 96000], "resolution_mm": 1000, "nx": 97, "ny": 97}


class Out:
    def __init__(self):
        self.instances = []

    def instance(self, key, asset, at, face=None, **keys):
        self.instances.append((key, asset, at, face, keys))


class Named(list):
    name = "open"


class FakeStep:
    def __init__(self, params, chance=1.0, keep_out=0.0, keep_out_field=True):
        self.params = params
        self.grid = GRID
        self.seed = "00000000000000a1"
        self.id = "groves"
        self.chance, self.ko, self.has_ko = chance, keep_out, keep_out_field
        self.out = Out()
        self.formulas = []
        self.warnings = []

    def fields(self, names, view="full", window=None):
        if not self.has_ko:
            raise RpcError(-32602, "no field", {"param": "fields"})
        return [Named()]

    def eval(self, formula, x=None, y=None, view="full"):
        self.formulas.append(formula)
        if formula.startswith("= f('open')"):
            return [self.ko] * len(x)
        return [self.chance * (1 - self.ko)] * len(x)

    def pack(self, select="*", columns=()):
        if "bounds" in columns:
            ids = sorted(select)
            return {"id": ids, "bounds": [[-600, -600, 0, 600, 600, 9000] for _ in ids]}
        return {"id": ["tree_pine", "tree_oak"] if select == "@plant/tree" else []}

    def instances(self, select="*"):
        return {"x": [48000], "y": [48000]}

    def warning(self, code, subject, message, hint, **kw):
        self.warnings.append(code)


P = {"assets": ["@plant/tree"], "spacing": 12, "density": "= f('cover')", "members": [2, 5],
     "radius": 6.5, "shrink": 0.4, "scale": [0.9, 1.35]}


class Draws(unittest.TestCase):
    def test_draw_is_fixed_and_in_range(self):
        a = C["draw"]("ab", "s12000_0_0", "keep")
        self.assertEqual(a, C["draw"]("ab", "s12000_0_0", "keep"))
        self.assertNotEqual(a, C["draw"]("ab", "s12000_0_1", "keep"))
        self.assertTrue(all(0 <= C["draw"]("ab", str(k), "x") < 1 for k in range(200)))

    def test_pick_by_weight(self):
        w = [("a", 3.0), ("b", 1.0)]
        self.assertEqual(C["pick"](w, 0.0), "a")
        self.assertEqual(C["pick"](w, 0.74), "a")
        self.assertEqual(C["pick"](w, 0.76), "b")


class Places(unittest.TestCase):
    def test_one_candidate_per_cell_keyed_by_place(self):
        p = C["Params"](P, GRID)
        cands = C["candidates"](p, GRID, "ab")
        self.assertEqual(len(cands), 64)        # 96 m / 12 m: 8 cells each way
        for c in cands:
            _, i, j = c["key"].split("_")
            self.assertTrue(int(i) * 12000 <= c["x"] < (int(i) + 1) * 12000)
            self.assertTrue(int(j) * 12000 <= c["y"] < (int(j) + 1) * 12000)

    def test_members_in_disc_and_named_after_parent(self):
        p = C["Params"](P, GRID)
        par = {"key": "s12000_4_4", "x": 48000, "y": 48000}
        ms = C["group"](p, GRID, "ab", par)
        self.assertTrue(2 <= len(ms) <= 5)
        for k, m in enumerate(ms):
            self.assertEqual(m["key"], f"s12000_4_4_m{k}")
            self.assertLessEqual((m["x"] - 48000) ** 2 + (m["y"] - 48000) ** 2, 6500 ** 2 + 2)
            self.assertLessEqual(m["scale"], 1.35)

    def test_bad_params_refused(self):
        with self.assertRaises(ValueError):
            C["Params"](dict(P, shrink=1), GRID)
        with self.assertRaises(ValueError):
            C["Params"](dict(P, members=[3, 1]), GRID)


class Generate(unittest.TestCase):
    def test_bad_param_said_at_its_key(self):
        errors = []
        s = FakeStep(dict(P, shrink=1.5))
        s.error = lambda code, subject, message, hint, **kw: errors.append((code, subject, message, hint, kw))
        C["generate"](s)
        self.assertEqual(len(errors), 1)
        code, subject, message, hint, kw = errors[0]
        self.assertEqual((code, subject), ("params", "step.groves.params.shrink"))
        self.assertIn("below 1", message)
        self.assertTrue(hint and kw.get("explain"))
        self.assertEqual(s.out.instances, [])

    def test_members_take_the_parents_asset(self):
        s = FakeStep(P)
        C["generate"](s)
        by_key = {k: a for k, a, *_ in s.out.instances}
        self.assertTrue(by_key)
        # A parent that gave way leaves its members (rules.md 7.3); where it stands, they share
        # its asset.
        for k, a in by_key.items():
            if "_m" in k and k.split("_m")[0] in by_key:
                self.assertEqual(a, by_key[k.split("_m")[0]])
        # No two kept things stand within `gap` of each other.
        pts = [(x, y) for _, _, (x, y), *_ in s.out.instances] if s.out.instances and isinstance(s.out.instances[0][2], tuple) else []
        for i, a in enumerate(pts):
            for b in pts[i + 1:]:
                self.assertGreaterEqual((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2, 2400 ** 2)
        self.assertTrue(all(a in ("tree_pine", "tree_oak") for a in by_key.values()))
        self.assertIn("* (1 - f('open'))", s.formulas[0])

    def test_same_seed_same_world(self):
        a, b = FakeStep(P), FakeStep(P)
        C["generate"](a)
        C["generate"](b)
        self.assertEqual(a.out.instances, b.out.instances)

    def test_kept_out_everywhere_is_empty(self):
        s = FakeStep(P, keep_out=1.0)
        C["generate"](s)
        self.assertEqual(s.out.instances, [])
        self.assertEqual(s.warnings, ["empty"])

    def test_partly_kept_out_returns_nothing_in_it(self):
        # Draws keep some things where keep_out is 0.3, but the core would find each of them in
        # keep_out (rules.md 8.3), so the step drops them all.
        s = FakeStep(P, keep_out=0.3)
        C["generate"](s)
        self.assertEqual(s.out.instances, [])

    def test_no_keep_out_field(self):
        s = FakeStep(P, keep_out_field=False)
        C["generate"](s)
        self.assertEqual(s.formulas[0], "= clamp(f('cover'), 0, 1)")

    def test_around_thins_far_from_the_other(self):
        s = FakeStep(dict(P, members=0, around={"of": "trees", "radius": 20}))
        C["generate"](s)
        for _, _, (x, y), _, _ in s.out.instances:
            self.assertLess((x - 48000) ** 2 + (y - 48000) ** 2, 20000 ** 2)


if __name__ == "__main__":
    unittest.main()
