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


class Clearing(list):
    name = "clearing"


class Out:
    def __init__(self):
        self.features, self.instances = {}, []

    def feature(self, key, kind, at=None, path=None, region=None, **attrs):
        self.features[key] = (kind, path)
        self.attrs = getattr(self, "attrs", {})
        self.attrs[key] = attrs

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

    keep_out = None                                    # a function of (x, y) in metres, or None

    def fields(self, names, view="full", window=None):
        if names == ["role:keep_out"]:                 # as the engine: no such field, none listed
            return [] if self.keep_out is None else [Clearing()]
        if self.height is None:
            raise LookupError("no surface")
        return [Named()]

    def eval(self, formula, x=None, y=None, view="full"):
        self.evals += 1
        fn = self.keep_out if "clearing" in formula else self.height
        return [fn(a / 1000.0, b / 1000.0) for a, b in zip(x, y)]

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
            self.assertGreater(S["_length"](pts), 1, k)        # none of no length (a street on the shore)

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

    def test_ends_on_the_outline_not_past_it(self):
        # quay's walk: contour streets ran a step past the district into the fields (the network
        # clips, but bed and paint read the whole path). An end outside the district lies on its
        # outline, 5 mm past the crossing, so the clip still makes the junction.
        step = run_streets(lambda x, y: 0.15 * y + 6 * math.sin(x / 70.0))
        ends = 0
        for k, (_, path) in step.out.features.items():
            for end in (path[0], path[-1]):
                q = (end[0] / 1000.0, end[1] / 1000.0)
                if not S["inside"](DISTRICT, q):
                    ends += 1
                    edge = min(H["_seg_dist"](a, b, q) for a, b in zip(DISTRICT, DISTRICT[1:] + DISTRICT[:1]))
                    self.assertLessEqual(edge, 0.0065, (k, q))
        self.assertGreater(ends, 4)

    def test_length_written(self):
        # Every street carries `length`, its returned path's length in metres, for the bed's taper.
        step = run_streets(lambda x, y: 0.15 * y)
        for k, (_, path) in step.out.features.items():
            self.assertAlmostEqual(step.out.attrs[k]["length"], S["_length"](metres(path)), places=3)

    def test_contour_ends_on_a_way_beside_it_and_crosses_one_across(self):
        # A main way along the contour at y = 145 east of x = 300: the contour street at y = 140
        # ends on it instead of running beside it 5 m off. A main way down the fall at x = 200:
        # the contour streets run across it (the network nodes the crossing).
        fl = district()
        fl["kind:main_street"] = feature_list([
            ("beside", 1, [(300, 145), (440, 145)], "feature"),
            ("down", 1, [(200, 60), (200, 380)], "feature")])
        step = FakeStep({"district": "old_town", "origin": [250, 220]},
                        height=lambda x, y: 0.15 * y, features=fl)
        S["generate"](step)
        low = metres(step.out.features["sm2"][1])
        self.assertAlmostEqual(min(abs(p[1] - 140) for p in low), 0, delta=0.05)
        east = max(low, key=lambda p: p[0])
        self.assertLess(H["_seg_dist"]((300, 145), (440, 145), east), 0.001)
        self.assertLess(east[0], 301)
        for k in ("s0", "s1", "sm1"):
            xs = [p[0] for p in metres(step.out.features[k][1])]
            self.assertLess(min(xs), 190, k)
            self.assertGreater(max(xs), 210, k)

    def test_far_edge_edit_turns_no_seed_line(self):
        # quay's `unrelated`: the district's east edge comes in 40 m on land whose fall turns from
        # west to east. With `origin` written, the fall is read near the origin, so the seed line,
        # and the contour streets' western halves, stay where they were. Without it, the
        # district's mean gradient turns and every long street moves.
        def h(x, y):
            return 0.15 * y + 0.00025 * (x - 60) ** 2

        def run(east, origin):
            fl = district()
            ring = [(60, 60), (east, 60), (east, 380), (60, 380)]
            fl["old_town"] = feature_list([("old_town", 2, ring, "feature")])
            params = {"district": "old_town", "spacing": 50}
            if origin:
                params["origin"] = [200, 220]
            step = FakeStep(params, height=h, features=fl)
            S["generate"](step)
            return {k: metres(v[1]) for k, v in step.out.features.items() if k.startswith("s")}

        def west_off(a, b):                         # metres a's points west of x = 300 lie off b's
            return max((S["_dist_to"](b[k], p) if k in b else math.inf)
                       for k in a for p in a[k] if p[0] < 300)

        a, b = run(440, True), run(400, True)
        self.assertTrue(a)
        self.assertLess(west_off(a, b), 0.3)        # the thinning's tolerance, 0.25 m
        self.assertGreater(west_off(*[run(e, False) for e in (440, 400)]), 1)

    def test_down_street_beside_the_way_dropped(self):
        # quay's doubled ends: a down street within spacing / 2 of the main way to the quay is not
        # laid; the way goes down there already. Without the way one is laid there.
        def run(ways):
            fl = district()
            fl["quay"] = feature_list([("quay", 1, [(40, 60), (480, 60)], "feature")])
            if ways:
                fl["kind:main_street"] = feature_list([("way", 1, [(290, 60), (290, 380)], "feature")])
            step = FakeStep({"district": "old_town", "shore": "quay", "origin": [250, 250]},
                            height=lambda x, y: 0.15 * y, features=fl)
            S["generate"](step)
            return {k: metres(v[1]) for k, v in step.out.features.items() if k.startswith("d")}
        near = lambda downs: [k for k, p in downs.items() if min(abs(q[0] - 290) for q in p) < 20]  # noqa: E731
        self.assertTrue(near(run(False)))
        with_way = run(True)
        self.assertTrue(with_way)
        self.assertEqual(near(with_way), [])

    def test_down_street_crossing_a_way_ends_on_it(self):
        # Step 6: a down street that crosses a main way ends where it first meets it, instead of
        # being dropped; one that only passes near the way is still not laid.
        p = S["Params"]({"district": "old_town", "cross": 90, "spacing": 40})
        land = S["Land"].flat(DISTRICT, 4)
        shore = [(40, 60), (480, 60)]
        longs = {0: [(60, 100), (440, 100)]}
        way = [(130, 40), (80, 120)]                        # crosses x = 105 at y = 80
        downs = S["down_to"](p, DISTRICT, land, longs, shore, [way])
        self.assertEqual(downs["d45"], [(105, 100), (105.0, 80.0)])
        self.assertIn("d135", downs)                        # 30 m off the way: laid to the shore
        self.assertEqual(downs["d135"][-1], (195.0, 60.0))
        bend = S["down_to"](p, DISTRICT, land, {0: [(60, 100), (440, 100)]}, shore,
                            [[(100, 100), (110, 100), (110, 60)]])   # starts on the way's bend
        self.assertNotIn("d45", bend)
        near = [(118, 40), (118, 120)]                      # 13 m from x = 105, not crossing it
        downs = S["down_to"](p, DISTRICT, land, {0: [(60, 100), (440, 100)]}, shore, [near])
        self.assertNotIn("d45", downs)

    def test_free_end_runs_on_to_a_street_or_way(self):
        # Step 8: an end 10 m short of another street, heading for it, runs on and meets it at a
        # vertex written into both; one heading away, or farther than spacing / 2, stays.
        p = S["Params"]({"district": "old_town", "spacing": 40})
        streets = {"a": [(100, 100), (200, 100)], "b": [(150, 200), (150, 110)]}
        out = S["join_ends"](p, DISTRICT, streets)
        self.assertEqual(out["b"][-1], (150.0, 100.0))
        self.assertIn((150.0, 100.0), out["a"])
        away = S["join_ends"](p, DISTRICT, {"a": [(100, 100), (200, 100)],     # beside it, 10 m
                                            "b": [(150, 110), (250, 110)]})   # off: not ahead
        self.assertEqual(away["b"], [(150, 110), (250, 110)])
        far = S["join_ends"](p, DISTRICT, {"a": [(100, 100), (200, 100)],
                                           "b": [(150, 200), (150, 125)]})
        self.assertEqual(far["b"][-1], (150, 125))
        way = S["join_ends"](p, DISTRICT, {"b": [(150, 200), (150, 112)]},
                             ways=[[(60, 100), (440, 100)]])
        self.assertEqual(way["b"][-1], (150.0, 100.0))

    def test_end_on_the_outline_or_a_street_not_joined(self):
        p = S["Params"]({"district": "old_town", "spacing": 40})
        streets = {"a": [(100, 100), (200, 100)], "b": [(150, 300), (150, 100)],
                   "c": [(300, 370), (300, 380.005)]}            # past the outline: cut there
        out = S["join_ends"](p, DISTRICT, {k: list(v) for k, v in streets.items()},
                             ways=[[(60, 385), (440, 385)]])
        self.assertEqual(out, streets)

    def test_lowest_contour_leaves_two_lots_to_the_shore(self):
        # quay's waterfront had no houses: the lowest contour lay 26 to 29 m above the shore and
        # its 24 m lots took the band. With `lot_depth`, no long street comes nearer the shore than
        # two lot depths; the streets down to the shore still reach it.
        def run(**extra):
            fl = district()
            fl["quay"] = feature_list([("quay", 1, [(40, 60), (480, 60)], "feature")])
            step = FakeStep(dict({"district": "old_town", "shore": "quay", "origin": [250, 220],
                                  "spacing": 50}, **extra),
                            height=lambda x, y: 0.15 * y, features=fl)
            S["generate"](step)
            return step.out.features
        def nearest(out):
            return min(p[1] - 60 for k, v in out.items() if k.startswith("s")
                       for p in metres(v[1]) if S["inside"](DISTRICT, p))
        self.assertLess(nearest(run()), 48)
        out = run(lot_depth=24)
        self.assertGreaterEqual(nearest(out), 48)
        self.assertTrue(any(k.startswith("d") for k in out))

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
          "house_row": [-3500, -4000, 0, 3500, 4000, 7000],
          "house_low": [-4000, -3000, 0, 4000, 3000, 4000],
          "house_wide": [-6000, -4000, 0, 6000, 4000, 6000]}


def pack_table(select):
    ids = [a for a in select if a in BOUNDS]
    return {"id": ids, "bounds": [BOUNDS[a] for a in ids]}


def _two_piece_lot():
    """One cell `lots#cp_0` of two pieces: an 11 m by 16 m lot on the street, and a 2 m sliver."""
    big = [(20, 3), (31.01, 3), (31.01, 19), (20, 19)]
    sliver = [(40, 3), (42, 3), (42, 5), (40, 5)]
    coords, starts = [], [0]
    for ring in (big, sliver):
        coords.extend([int(round(x * 1000)), int(round(y * 1000))] for x, y in ring)
        starts.append(len(coords))
    return _FL({"name": ["lots#cp_0"], "geometry": [2], "pieces": [0, 2], "rings": [0, 1, 2],
                "ring_starts": starts, "coords": coords, "class": ["cell"]})


class FakeHouses(FakeStep):
    def pack(self, select="*", columns=()):
        return pack_table(select)


def houses_step(cells, height=None, keep_out=None, **params):
    fl = {"lots": feature_list(cells),
          "kind:main_street": feature_list([("streets#a", 1, [(0, 0), (100, 0)], "feature")]),
          "kind:street": feature_list([("streets#b", 1, [(0, 0), (0, 100)], "feature")])}
    step = FakeHouses(dict({"lots": "lots"}, **params), height=height, features=fl)
    step.keep_out = keep_out
    step.id = "houses"
    H["generate"](step)
    return step


class HousesTest(unittest.TestCase):
    def test_faces_its_street(self):
        # A lot 8.6 m along the east-west street at y = 0, 16 m deep, north of it: the house's front
        # faces south (180), its front 1 m in from the frontage. Only house_narrow fits 7.6 m.
        lot = [(20, 3), (28.6, 3), (28.6, 19), (20, 19)]
        step = houses_step([("lots#cab12_3", 2, lot, "cell"), ("lots#bzz_0", 2, lot, "rest")])
        self.assertEqual(len(step.out.instances), 1)
        key, asset, at, face, keys = step.out.instances[0]
        self.assertEqual((key, asset, face), ("hab12_3", "house_narrow", 180.0))
        self.assertEqual(at, (24300, 3000 + 1000 + 4000))
        self.assertEqual(keys, {})

    def test_faces_the_nearer_street_and_ground(self):
        # A lot beside the north-south street at x = 0: the house faces west (270).
        lot = [(3, 40), (19, 40), (19, 48.6), (3, 48.6)]
        step = houses_step([("lots#cq_0", 2, lot, "cell")], ground="pad")
        key, asset, at, face, keys = step.out.instances[0]
        self.assertEqual(face, 270.0)
        self.assertEqual(at, (3000 + 1000 + 4000, 44300))
        self.assertEqual(keys, {"ground": "pad"})

    def test_prefers_in_order_and_skips_small(self):
        wide = [(40, 3), (52, 3), (52, 10), (40, 10)]        # 12 m front, 7 m deep: only house_low
        tiny = [(60, 3), (64, 3), (64, 20), (60, 20)]        # 4 m front: nothing fits
        step = houses_step([("lots#cw_0", 2, wide, "cell"), ("lots#ct_0", 2, tiny, "cell")])
        self.assertEqual([i[1] for i in step.out.instances], ["house_low"])
        small = [s for s in step.said if s[1] == "small"]
        self.assertEqual(small[0][:3], ("info", "small", "step.houses"))
        self.assertEqual(small[0][3]["count"], 1)

    def test_largest_that_fits(self):
        # quay never placed house_wide, written second: the largest footprint that fits is taken,
        # whatever the order written. A 13 m lot takes house_wide (12 m), a 9 m lot house_low (8 m,
        # larger than house_narrow), whichever is written first.
        wide = [(20, 3), (33, 3), (33, 19), (20, 19)]
        mid = [(40, 3), (49, 3), (49, 19), (40, 19)]
        step = houses_step([("lots#cw_0", 2, wide, "cell"), ("lots#cm_0", 2, mid, "cell")],
                           assets=["house_narrow", "house_low", "house_wide"])
        self.assertEqual([i[1] for i in step.out.instances], ["house_wide", "house_low"])

    def test_max_relief_skips_steep_lots_unless_padded(self):
        # Until pads work above a step that reads the ground, a house on more relief than
        # `max_relief` is left out (counted, `steep`); with `ground` the pad levels it and relief is
        # not read. Land rises 0.5 a metre north from x = 40: the east lot spans 6 m of it.
        h = lambda x, y: 0.0 if x < 40 else 0.5 * y  # noqa: E731
        flat = [(20, 3), (28.6, 3), (28.6, 19), (20, 19)]
        steep = [(50, 3), (58.6, 3), (58.6, 19), (50, 19)]
        cells = [("lots#cf_0", 2, flat, "cell"), ("lots#cs_0", 2, steep, "cell")]
        step = houses_step(cells, height=h, max_relief=1.5)
        self.assertEqual([i[0] for i in step.out.instances], ["hf_0"])
        self.assertEqual(step.evals, 1)                       # one batch for every house (R23)
        info = [s for s in step.said if s[1] == "steep"]
        self.assertEqual(info[0][3], {"count": 1, "houses": ["hs_0"]})
        step = houses_step(cells, height=h, max_relief=1.5, ground="pad")
        self.assertEqual([i[0] for i in step.out.instances], ["hf_0", "hs_0"])
        self.assertEqual(step.evals, 0)
        self.assertTrue(all(i[4] == {"ground": "pad"} for i in step.out.instances))

    def test_clockwise_ring_and_closing_point(self):
        lot = [(20, 3), (20, 19), (29, 19), (29, 3), (20, 3)]
        step = houses_step([("lots#cab12_3", 2, lot, "cell")])
        self.assertEqual(step.out.instances[0][3], 180.0)


class RowsTest(unittest.TestCase):
    """Rules 2 and 4: a lot's frontage holds a row of houses wall to wall, pushed back clear of the
    street's keep_out, the setback following the street's own width."""

    def test_row_fills_the_frontage(self):
        # An 11.01 m frontage on the street at y = 0: two house_narrow (5 m) wall to wall (1 cm
        # apart, PARTY), 0.5 m gaps at the lot's edges, named by the lot and their place in the row.
        lot = [(20, 3), (31.01, 3), (31.01, 19), (20, 19)]
        step = houses_step([("lots#cab_3", 2, lot, "cell")])
        got = sorted((k, a, at) for k, a, at, _, _ in step.out.instances)
        self.assertEqual(got, [("hab_3", "house_narrow", (23000, 8000)),       # from the chord's
                               ("hab_3_1", "house_narrow", (28010, 8000))])    # start, west
        rows = [s for s in step.said if s[1] == "rows"][0][3]
        self.assertEqual((rows["houses"], rows["lots_held"], rows["lots"]), (2, 1, 1))

    def test_row_covers_most_mixed(self):
        # 13 m less two 0.5 m gaps: house_narrow + house_row (12 m) covers more than two narrow.
        lot = [(20, 3), (33.01, 3), (33.01, 19), (20, 19)]
        step = houses_step([("lots#cm_0", 2, lot, "cell")], assets=["house_narrow", "house_row"])
        self.assertEqual(sorted(i[1] for i in step.out.instances), ["house_narrow", "house_row"])
        xs = sorted(i[2][0] for i in step.out.instances)
        self.assertEqual(xs[1] - xs[0], 6010)             # 2.5 + 3.5 and PARTY: wall to wall

    def test_keep_out_pushes_the_row_back(self):
        # The street's keep_out reaches 4.2 m from its centre line (y = 0): the row's front (each
        # house's +y bound, 4 m from its origin) stands at the first 0.5 m step past it, 4.5 m.
        lot = [(20, 0), (31.01, 0), (31.01, 19), (20, 19)]
        step = houses_step([("lots#ck_0", 2, lot, "cell")], setback=1, assets=["house_narrow"],
                           keep_out=lambda x, y: 1.0 if y < 4.2 else 0.0)
        self.assertEqual({i[2][1] for i in step.out.instances}, {4500 + 4000})
        self.assertEqual(len(step.out.instances), 2)
        self.assertEqual(sum(1 for _ in range(step.evals)), 1)   # one batch for every lot (R23)

    def test_corner_house_beside_a_cross_street_dropped(self):
        # A corner lot: a cross street's keep_out covers x < 24.5 at any depth. Pushing back does not
        # clear the west house, so it is dropped and the gap falls at the junction.
        lot = [(20, 3), (31.01, 3), (31.01, 19), (20, 19)]
        step = houses_step([("lots#cc_0", 2, lot, "cell")], assets=["house_narrow"],
                           keep_out=lambda x, y: 1.0 if x < 24.5 else 0.0)
        self.assertEqual([(i[0], i[2]) for i in step.out.instances], [("hc_0_1", (28010, 8000))])

    def test_wedge_lot_takes_a_shorter_row(self):
        # The lot's back narrows to 7 m: two narrow houses' back corners stand outside it, so the
        # row of one is taken; narrowed to 3 m, none fits.
        lot = [(20, 3), (31.01, 3), (29, 13), (22, 13)]
        step = houses_step([("lots#cw_0", 2, lot, "cell")], assets=["house_narrow"])
        self.assertEqual([i[0] for i in step.out.instances], ["hw_0"])
        lot = [(20, 3), (31, 3), (27, 13), (24, 13)]
        step = houses_step([("lots#cw_0", 2, lot, "cell")], assets=["house_narrow"])
        self.assertEqual(len(step.out.instances), 0)

    def test_corner_triangle_takes_a_row_at_its_square_end(self):
        # A corner lot cut as a right triangle, its square corner at the frontage's start (west):
        # centred, the row's back corners stand outside; set against the start, one house fits.
        lot = [(20, 3), (36, 3), (20, 19)]
        step = houses_step([("lots#cq_0", 2, lot, "cell")], assets=["house_narrow"])
        self.assertEqual([(i[0], i[2]) for i in step.out.instances], [("hq_0", (23000, 8000))])

    def test_terrace_under_each_run(self):
        # With `terrace`, a run of houses stands on one region of that kind, named after the lot and
        # the run's first house; the houses take no ground. A corner house dropped by keep_out
        # leaves a run starting at place 1: `tc_0_1`.
        lot = [(20, 3), (31.01, 3), (31.01, 19), (20, 19)]
        step = houses_step([("lots#ct_0", 2, lot, "cell")], assets=["house_narrow"],
                           terrace="terrace")
        self.assertEqual(sorted(step.out.features), ["tt_0"])
        kind, _ = step.out.features["tt_0"]
        self.assertEqual(kind, "terrace")
        self.assertTrue(all(i[4] == {} for i in step.out.instances))
        rows = [x for x in step.said if x[1] == "rows"][0][3]
        self.assertEqual((rows["houses"], rows["terraces"]), (2, 1))
        step = houses_step([("lots#cc_0", 2, lot, "cell")], assets=["house_narrow"],
                           terrace="terrace", keep_out=lambda x, y: 1.0 if x < 24.5 else 0.0)
        self.assertEqual(sorted(step.out.features), ["tc_0_1"])

    def test_terrace_outline_covers_the_run(self):
        lot = [(20, 3), (31.01, 3), (31.01, 19), (20, 19)]
        frame = H["lot_frame"](lot, [((0, 0), (100, 0))])
        sizes = [("house_narrow", -2.5, -4, 2.5, 4)]
        row = H["fit_row"](lot, frame, sizes, 1.0, 0.5)
        ring = H["run_outline"](row)
        xs, ys = sorted({round(x, 3) for x, _ in ring}), sorted({round(y, 3) for _, y in ring})
        self.assertEqual((xs, ys), ([20.5, 30.51], [4.0, 12.0]))
        self.assertGreater(H["_area2"](ring), 0)            # counter-clockwise

    def test_terrace_run_past_max_cut_left_out(self):
        # Land falling 1 m a metre east: a 10 m run's terrace cuts about 5 m at its ends.
        lot = [(20, 3), (31.01, 3), (31.01, 19), (20, 19)]
        step = houses_step([("lots#cs_0", 2, lot, "cell")], height=lambda x, y: -x,
                           assets=["house_narrow"], terrace="terrace", max_cut=3, grade=0.7)
        self.assertEqual(step.out.instances, [])
        steep = [x for x in step.said if x[1] == "steep"][0][3]
        self.assertEqual(steep["count"], 2)

    def test_lot_of_two_pieces_stands_for_its_largest_and_is_said(self):
        fl = {"lots": _two_piece_lot(),
              "kind:main_street": feature_list([("streets#a", 1, [(0, 0), (100, 0)], "feature")]),
              "kind:street": feature_list([("streets#b", 1, [(0, 0), (0, 100)], "feature")])}
        step = FakeHouses({"lots": "lots", "assets": ["house_narrow"]}, features=fl)
        step.id = "houses"
        H["generate"](step)
        self.assertEqual(len(step.out.instances), 2)            # on the 11 m piece, not the sliver
        split = [x for x in step.said if x[1] == "split"][0][3]
        self.assertEqual(split, {"count": 1, "lots": ["lots#cp_0"]})

    def test_fallback_fills_a_shallow_corner(self):
        # Rule 2b: a lot 7 m deep takes no 8 m deep house_narrow; with `fallback` it takes a
        # house_low (6 m deep) instead of standing empty.
        lot = [(20, 3), (30, 3), (30, 10), (20, 10)]
        step = houses_step([("lots#cs_0", 2, lot, "cell")], assets=["house_narrow"], setback=0.5)
        self.assertEqual(step.out.instances, [])
        step = houses_step([("lots#cs_0", 2, lot, "cell")], assets=["house_narrow"], setback=0.5,
                           fallback=["house_low"])
        self.assertEqual([i[1] for i in step.out.instances], ["house_low"])

    def test_terrace_level_at_the_footway(self):
        # Rule 8: on land rising north from a street along y = 0, the terrace is written at the
        # footway's height in front of the doors, not the land's mean under the run, and it
        # carries the door apron out to the footway.
        lot = [(20, 3), (31.01, 3), (31.01, 19), (20, 19)]
        step = houses_step([("lots#cf_0", 2, lot, "cell")], height=lambda x, y: 0.1 * y,
                           assets=["house_narrow"], terrace="terrace", max_cut=3, grade=0.7)
        attrs = step.out.attrs["tf_0"]
        self.assertEqual(attrs["set_level"], 1)
        fronts = H["fronts"]([(0, h) for _, h in H["fit_row"](
            lot, H["lot_frame"](lot, [((0, 0), (100, 0))]), [("house_narrow", -2.5, -4, 2.5, 4)],
            1.0, 0.5)])
        foot = [0.1 * (m[1] + f[1] * H["FOOTWAY_OUT"]) for m, f in fronts]
        self.assertAlmostEqual(attrs["level"], sum(foot) / len(foot), places=3)

    def test_steep_footway_steps_the_row(self):
        # A footway climbing 1 m a metre along the row: each house its own floor (a stepped
        # terrace), each within half of STEP_SPAN of its own footway.
        run = [(i, ("n", (0, 0), 0, [(5 * i, 0), (5 * i + 5, 0), (5 * i + 5, 8), (5 * i, 8)]))
               for i in range(3)]
        pieces = H["split_by_level"](run, [0.0, 0.3, 0.5])
        self.assertEqual([([i for i, _ in r], z) for r, z in pieces], [([0, 1, 2], 0.25)])
        pieces = H["split_by_level"](run, [0.0, 1.0, 2.0])
        self.assertEqual([([i for i, _ in r], z) for r, z in pieces],
                         [([0], 0.0), ([1], 1.0), ([2], 2.0)])

    def test_terrace_and_ground_refused(self):
        step = houses_step([], terrace="terrace", ground="pad")
        self.assertEqual(step.said[0][:2], ("error", "params"))

    def test_rows_by_cover_order(self):
        sizes = [("n", -2.5, -4, 2.5, 4), ("r", -3.5, -4, 3.5, 4)]
        rows = H["rows_by_cover"](12.01, sizes, 9.0)
        self.assertEqual(rows[0], (0, 1))                 # 12 m and a PARTY
        self.assertEqual(rows[1], (0, 0))                 # 10 m and a PARTY
        self.assertNotIn((0, 1), H["rows_by_cover"](12.0, sizes, 9.0))   # no room for the PARTY
        self.assertEqual(H["rows_by_cover"](12.0, sizes, 7.0), [])   # too deep for every asset


if __name__ == "__main__":
    unittest.main()
