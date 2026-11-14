"""Tests for streets.py and houses.py, with a stand-in for the SDK's ``step``.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/town -p 'test_*.py'

The engine's side of the contract (geometry.md 8: `build.features` naming a cells table returns its
cells and rest with a `class` column) is not built yet, so the stand-in answers it as the contract
says. Heights come from a Python function the stand-in evaluates at the asked points.
"""

import math
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
S = runpy.run_path(os.path.join(HERE, "streets.py"))
H = runpy.run_path(os.path.join(HERE, "houses.py"))
GRID = {"extent_mm": [0, 0, 512000, 512000], "resolution_mm": 2000, "nx": 257, "ny": 257}
DISTRICT = [(60, 60), (440, 60), (440, 380), (60, 380)]


def feature_list(shapes):
    """A FeatureList of (name, geometry, [ring]) with one piece each; coordinates in mm."""
    coords, pieces, rings, starts, names, geom, cls = [], [0], [0], [0], [], [], []
    for name, g, ring, c in shapes:
        coords.extend([int(round(x * 1000)), int(round(y * 1000))] for x, y in ring)
        starts.append(len(coords))
        rings.append(len(starts) - 1)
        pieces.append(len(rings) - 1)
        names.append(name)
        geom.append(g)
        cls.append(c)
    return _FL({"name": names, "geometry": geom, "pieces": pieces, "rings": rings,
                "ring_starts": starts, "coords": coords, "class": cls})


class _FL(dict):
    def __len__(self):
        return len(self["geometry"])

    @property
    def names(self):
        return list(self["name"])

    def rings(self, f):
        out = []
        for p in range(self["pieces"][f], self["pieces"][f + 1]):
            out.append([self["coords"][self["ring_starts"][r]:self["ring_starts"][r + 1]]
                        for r in range(self["rings"][p], self["rings"][p + 1])])
        return out


class Named(list):
    name = "height"


class Out:
    def __init__(self):
        self.features, self.instances = {}, []

    def feature(self, key, kind, at=None, path=None, region=None, **attrs):
        self.features[key] = (kind, path)

    def instance(self, key, asset, at, face=None, **keys):
        self.instances.append((key, asset, at, face, keys))


class FakeStep:
    def __init__(self, params, height=None, features=None, pack=None):
        self.params, self.grid, self.id, self.seed = params, GRID, "streets", "00"
        self.height = height
        self.fl = features or {}
        self.packt = pack
        self.out = Out()
        self.said = []
        self.evals = 0

    def fields(self, names, view="full", window=None):
        if self.height is None:
            raise LookupError("no surface")
        return [Named()]

    def eval(self, formula, x=None, y=None, view="full"):
        self.evals += 1
        return [self.height(a / 1000.0, b / 1000.0) for a, b in zip(x, y)]

    def features(self, select, within=None, columns=()):
        # As the engine (protocol.md 3.3): `kind:<k>` alone, or a list of names.
        if isinstance(select, list) and any(s.startswith("kind:") for s in select):
            raise ValueError(f"no feature is named `{select}`")
        key = select[0] if isinstance(select, list) and len(select) == 1 else select
        return self.fl[key if isinstance(key, str) else tuple(key)]

    def pack(self, select="*", columns=()):
        return self.packt

    def error(self, code, subject, message, hint, **kw):
        self.said.append(("error", code, subject))

    def warning(self, code, subject, message, hint, **kw):
        self.said.append(("warning", code, subject))

    def info(self, code, subject, message, hint, **kw):
        self.said.append(("info", code, subject, kw.get("data")))


def district(ring=DISTRICT):
    return {"old_town": feature_list([("old_town", 2, ring, "feature")])}


def run_streets(height, ring=DISTRICT, **params):
    step = FakeStep(dict({"district": "old_town"}, **params), height=height, features=district(ring))
    S["generate"](step)
    return step


def metres(path):
    return [(x / 1000.0, y / 1000.0) for x, y in path]


class StreetsTest(unittest.TestCase):
    def test_steep_land_contours(self):
        # A hillside rising north at 0.15 (8.5 degrees) with a swell: each long street keeps its
        # seed's height to within a few centimetres, so it follows the contour.
        def h(x, y):
            return 0.15 * y + 6 * math.sin(x / 70.0)
        step = run_streets(h)
        self.assertEqual(step.evals, 1)                       # one batch for the land (R23)
        longs = {k: metres(v[1]) for k, v in step.out.features.items() if k.startswith("s")}
        self.assertGreaterEqual(len(longs), 5)
        for key, pts in longs.items():
            inner = [p for p in pts if S["inside"](DISTRICT, p)]
            levels = [h(*p) for p in inner]
            self.assertLess(max(levels) - min(levels), 0.6, key)
            # not straight: a contour of the swell bends
            ys = [p[1] for p in inner]
            self.assertGreater(max(ys) - min(ys), 10, key)

    def test_flat_land_straight_along_bearing(self):
        step = run_streets(lambda x, y: 10.0, bearing=30)
        longs = [metres(v[1]) for k, v in step.out.features.items() if k.startswith("s")]
        self.assertTrue(longs)
        for pts in longs:
            a, b = pts[0], pts[-1]
            bearing = math.degrees(math.atan2(b[0] - a[0], b[1] - a[1])) % 180
            self.assertAlmostEqual(bearing, 30, delta=0.5)

    def test_every_end_a_junction_or_past_the_outline(self):
        # TODO "Dead ends tripled": every street ends on another street's vertex or outside the
        # district (where the network's `within` cuts it on the outline).
        step = run_streets(lambda x, y: 0.12 * y + 0.05 * x)
        paths = {k: v[1] for k, v in step.out.features.items()}
        verts = {k: set(p) for k, p in paths.items()}
        for k, p in paths.items():
            for end in (p[0], p[-1]):
                outside = not S["inside"](DISTRICT, (end[0] / 1000.0, end[1] / 1000.0))
                on_other = any(end in verts[o] for o in verts if o != k)
                loop = p[0] == p[-1]
                self.assertTrue(outside or on_other or loop, (k, end))
        self.assertTrue(any(k.startswith("x") for k in paths))   # cross streets exist

    def test_names_by_place(self):
        # Moving the district's far edge renames no long street near the centre.
        a = run_streets(lambda x, y: 0.15 * y)
        b = run_streets(lambda x, y: 0.15 * y, ring=[(60, 60), (470, 60), (470, 380), (60, 380)])
        keys_a = {k for k in a.out.features if k.startswith("s")}
        keys_b = {k for k in b.out.features if k.startswith("s")}
        self.assertEqual(keys_a, keys_b)

    def test_far_edge_moves_no_contour(self):
        # R13, asked by `quay`: an edit at the district's far edge moves its centre a little; the
        # seeds are snapped to whole spacings, so a contour street keeps its line, not only its name
        # (inside the old district: past the edit it is cut at the new outline).
        h = lambda x, y: 0.15 * y + 4 * math.sin(x / 60.0)  # noqa: E731
        a = run_streets(h)
        b = run_streets(h, ring=[(60, 60), (434, 60), (434, 380), (60, 380)])
        for k, v in a.out.features.items():
            if not k.startswith("s"):
                continue
            pa = [p for p in metres(v[1]) if p[0] < 420]
            pb = metres(b.out.features[k][1])
            self.assertLess(max(S["_dist_to"](pb, q) for q in pa), 0.6, k)   # two thinnings of 0.25

    def test_shore_streets_end_on_it(self):
        # `shore`: short streets from the lowest long street end exactly on the shore's line.
        fl = district()
        fl["quay"] = feature_list([("quay", 1, [(40, 60), (480, 60)], "feature")])
        step = FakeStep({"district": "old_town", "shore": "quay"}, height=lambda x, y: 0.15 * y,
                        features=fl)
        S["generate"](step)
        downs = {k: metres(v[1]) for k, v in step.out.features.items() if k.startswith("d")}
        self.assertTrue(downs)
        for k, pts in downs.items():
            self.assertAlmostEqual(pts[-1][1], 60.0, places=3, msg=k)

    def test_knoll_loops_once(self):
        # Round a knoll the contours close: each is one loop from its seed back to it, never a
        # spiral, and a loop too small for a cross street is not laid.
        def h(x, y):
            return 40 * math.exp(-((x - 250) ** 2 + (y - 220) ** 2) / (2 * 120 ** 2))
        step = run_streets(h)
        loops = [v[1] for k, v in step.out.features.items() if k.startswith("s") and v[1][0] == v[1][-1]]
        self.assertTrue(loops)
        for path in loops:
            pts = metres(path)
            self.assertGreaterEqual(S["_length"](pts), 180)
            angle = 0.0                               # winds once round the top
            for a, b in zip(pts, pts[1:]):
                angle += math.remainder(math.atan2(b[1] - 220, b[0] - 250) - math.atan2(a[1] - 220, a[0] - 250), 2 * math.pi)
            self.assertAlmostEqual(abs(angle), 2 * math.pi, delta=0.01)

    def test_rerun_identical(self):
        h = lambda x, y: 0.15 * y + 4 * math.cos(x / 50.0)  # noqa: E731
        self.assertEqual(run_streets(h).out.features, run_streets(h).out.features)

    def test_no_surface_warns_and_lays_straight(self):
        step = run_streets(None)
        self.assertIn(("warning", "no_surface", "step.streets"), step.said)
        self.assertTrue(step.out.features)

    def test_bad_params(self):
        step = FakeStep({"district": "old_town", "spacing": 0}, features=district())
        S["generate"](step)
        self.assertEqual(step.said, [("error", "params", "step.streets.params.spacing")])
        step = FakeStep({}, features=district())
        S["generate"](step)
        self.assertEqual(step.said, [("error", "params", "step.streets.params.district")])


# --- houses -------------------------------------------------------------------------------------

BOUNDS = {"house_narrow": [-2500, -4000, 0, 2500, 4000, 7000],
          "house_low": [-4000, -3000, 0, 4000, 3000, 4000]}


def pack_table(select):
    ids = [a for a in select if a in BOUNDS]
    return {"id": ids, "bounds": [BOUNDS[a] for a in ids]}


class FakeHouses(FakeStep):
    def pack(self, select="*", columns=()):
        return pack_table(select)


def houses_step(cells, **params):
    fl = {"lots": feature_list(cells),
          "kind:main_street": feature_list([("streets#a", 1, [(0, 0), (100, 0)], "feature")]),
          "kind:street": feature_list([("streets#b", 1, [(0, 0), (0, 100)], "feature")])}
    step = FakeHouses(dict({"lots": "lots"}, **params), features=fl)
    step.id = "houses"
    H["generate"](step)
    return step


class HousesTest(unittest.TestCase):
    def test_faces_its_street(self):
        # A lot 9 m along the east-west street at y = 0, 16 m deep, north of it: the house's front
        # faces south (180), its front 1 m in from the frontage.
        lot = [(20, 3), (29, 3), (29, 19), (20, 19)]
        step = houses_step([("lots#cab12_3", 2, lot, "cell"), ("lots#bzz_0", 2, lot, "rest")])
        self.assertEqual(len(step.out.instances), 1)
        key, asset, at, face, keys = step.out.instances[0]
        self.assertEqual((key, asset, face), ("hab12_3", "house_narrow", 180.0))
        self.assertEqual(at, (24500, 3000 + 1000 + 4000))
        self.assertEqual(keys, {})

    def test_faces_the_nearer_street_and_ground(self):
        # A lot beside the north-south street at x = 0: the house faces west (270).
        lot = [(3, 40), (19, 40), (19, 49), (3, 49)]
        step = houses_step([("lots#cq_0", 2, lot, "cell")], ground="pad")
        key, asset, at, face, keys = step.out.instances[0]
        self.assertEqual(face, 270.0)
        self.assertEqual(at, (3000 + 1000 + 4000, 44500))
        self.assertEqual(keys, {"ground": "pad"})

    def test_prefers_in_order_and_skips_small(self):
        wide = [(40, 3), (52, 3), (52, 10), (40, 10)]        # 12 m front, 7 m deep: only house_low
        tiny = [(60, 3), (64, 3), (64, 20), (60, 20)]        # 4 m front: nothing fits
        step = houses_step([("lots#cw_0", 2, wide, "cell"), ("lots#ct_0", 2, tiny, "cell")])
        self.assertEqual([i[1] for i in step.out.instances], ["house_low"])
        self.assertEqual(step.said[0][:3], ("info", "small", "step.houses"))
        self.assertEqual(step.said[0][3]["count"], 1)

    def test_clockwise_ring_and_closing_point(self):
        lot = [(20, 3), (20, 19), (29, 19), (29, 3), (20, 3)]
        step = houses_step([("lots#cab12_3", 2, lot, "cell")])
        self.assertEqual(step.out.instances[0][3], 180.0)


if __name__ == "__main__":
    unittest.main()
