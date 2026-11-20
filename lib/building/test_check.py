"""Tests for check.py: reach between surfaces and rooms, stairs, turns, the kit's pitch.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The meetings are surfaces.md 6.4's four rows, as `surface.meetings` gives them; the features are
what lay.py returns for that house (names as the build gives them, `lay#<key>`).
"""

import math
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
C = runpy.run_path(os.path.join(HERE, "check.py"))
L = runpy.run_path(os.path.join(HERE, "lay.py"))
P = C["params_of"]({})
ATTRS = {"storeys": 2, "storey_h": 3.0, "plinth": 0.3, "porch": 4, "walk_out": 12,
         "stair": {"t": 0.025, "width": 1, "run": 5}, "doors": [{"storey": 0, "t": 0.4, "width": 1}],
         "rooms": [{"storey": 0, "shares": [2, 1]}, {"storey": 1, "shares": [1, 1]}]}
HALL = [(20000, 20000), (32000, 20000), (32000, 28000), (20000, 28000)]


def house(attrs=None):
    a = dict(ATTRS)
    a.update(attrs or {})
    out = L["plan"]("hall", HALL, a, 10.0)
    feats = []
    for s in out["storeys"]:
        feats.append({"name": "lay#" + s["key"], "kind": "storey", "ring": s["region"], "path": s["region"],
                      "attrs": {"level": s["level"]}})
    for r in out["rooms"]:
        feats.append({"name": "lay#" + r["key"], "kind": "room", "ring": r["region"], "path": r["region"],
                      "attrs": {"storey": "lay#" + r["storey"]}})
    for f in out["flights"]:
        feats.append({"name": "lay#" + f["key"], "kind": "flight", "ring": f["path"], "path": f["path"],
                      "attrs": {"z0": f["z0"], "z1": f["z1"]}})
    for d in out["doorways"]:
        feats.append({"name": "lay#" + d["key"], "kind": "doorway", "ring": d["path"], "path": d["path"],
                      "attrs": {"sill_z": d["sill_z"]}})
    for p in out["footpaths"]:
        feats.append({"name": "lay#" + p["key"], "kind": "footpath", "ring": p["path"], "path": p["path"],
                      "attrs": {}})
    return feats


def row(a, b, x, y, gap=0, turn=None, ca=None, cb=None):
    return {"a": a, "b": b, "x": x, "y": y, "gap": gap, "turn": turn, "connector_a": ca, "connector_b": cb}


MEETINGS = [row("lay#hall_d0.end", "lay#hall_s0.floor", 32000, 24000, ca="foot"),
            row("lay#hall_f0.end", "lay#hall_s1.floor", 26000, 20600, ca="foot"),
            row("lay#hall_f0.start", "lay#hall_s0.floor", 21000, 20600, ca="foot"),
            row("lay#hall_p0.end", "lay#hall_d0.start", 36000, 24000, ca="foot", cb="foot")]


def codes(recs):
    return [(r[1], r[2]) for r in recs if r[0] != "info"]


class Reach(unittest.TestCase):
    def test_the_worked_example_is_reached_everywhere(self):
        self.assertEqual(codes(C["judge"](house(), MEETINGS, [], P)), [])

    def test_no_door_no_house(self):
        recs = C["judge"](house(), [m for m in MEETINGS if "d0.end" not in m["a"]], [], P)
        self.assertEqual(codes(recs), [("unreached", "lay#hall_s0"), ("unreached", "lay#hall_s1"),
                                       ("unreached", "lay#hall_s0_r0"), ("unreached", "lay#hall_s0_r1"),
                                       ("unreached", "lay#hall_s1_r0"), ("unreached", "lay#hall_s1_r1")])

    def test_a_stair_that_stops_short_leaves_the_upper_storey(self):
        """surfaces.md 6.4: z1 = 13.0 is `surface.gap` 300 mm; no walk crosses a gap."""
        short = [dict(m, gap=300) if m["a"] == "lay#hall_f0.end" else m for m in MEETINGS]
        self.assertEqual(codes(C["judge"](house(), short, [], P)),
                         [("unreached", "lay#hall_s1"), ("unreached", "lay#hall_s1_r0"),
                          ("unreached", "lay#hall_s1_r1")])

    def test_a_meeting_whose_connectors_differ_is_no_way_in(self):
        lane = [dict(m, connector_b="way") if m["a"] == "lay#hall_d0.end" else m for m in MEETINGS]
        self.assertEqual(codes(C["judge"](house(), lane, [], P))[0], ("unreached", "lay#hall_s0"))

    def test_rooms_cross_only_where_they_share_an_opening(self):
        """Three rooms in a row on the ground storey; the door enters the east one and the stair's
        foot is in the west one. With a 9 m opening asked for, no room shares that much, so the
        middle one is not reached."""
        feats = house({"rooms": [{"storey": 0, "shares": [1, 1, 1]}]})
        self.assertEqual(codes(C["judge"](feats, MEETINGS, [], P)), [])
        wide = C["params_of"]({"inner_door": 9})
        self.assertEqual(codes(C["judge"](feats, MEETINGS, [], wide)),
                         [("unreached", "lay#hall_s0_r1")])


# The test kit's pieces as pack bounds (mm): a 1 m bay and a 0.1 m filler, 0.3 m thick, their +y
# out of the wall; the corner a 0.3 m square; the leaf a door (not an obstacle).
KIT = {"wall": {"id": "bay", "pitch": 1000, "thick": 300, "width": 1000},
       "filler": {"id": "strip", "pitch": 100, "thick": 300, "width": 100},
       "corner": {"id": "corner", "pitch": 300, "thick": 300, "width": 300},
       "door": {"id": "leaf", "pitch": 1000, "thick": 300, "width": 1000}}
BOUNDS = {"bay": [-500, -150, 0, 500, 150, 3000], "strip": [-50, -150, 0, 50, 150, 3000],
          "corner": [-150, -150, 0, 150, 150, 3000]}


def members(out):
    """Where the core puts the plan's wall pieces (rules.md 5.1: `ends = "half"`, by arc length
    along the storey's ring, `offset` to the left), and its `at` pieces; leaves are left out."""
    rings = {s["key"]: s["region"] for s in out["storeys"]}
    level = {s["key"] + ".floor": s["level"] for s in out["storeys"]}
    got = []
    for r in out["runs"]:
        ring = rings[r["ref"]]
        segs = list(zip(ring, ring[1:] + ring[:1]))
        total = sum(math.dist(p, q) for p, q in segs)
        a, b = round(r["span"][0] * total), round(r["span"][1] * total)
        for k in range(r["count"]):
            s_ = a + ((2 * k + 1) * (b - a)) // (2 * r["count"])
            c = 0.0
            for p, q in segs:
                L = math.dist(p, q)
                if s_ < c + L:
                    e = ((q[0] - p[0]) / L, (q[1] - p[1]) / L)
                    u, off = s_ - c, r["offset"] * 1000
                    got.append({"name": f"lay#{r['key']}#r{k}", "x": round(p[0] + e[0] * u - e[1] * off),
                                "y": round(p[1] + e[1] * u + e[0] * off), "face": round(r["face"] * 1000),
                                "z": level[r["ref"] + ".floor"],
                                "bounds": BOUNDS[r["asset"]]})
                    break
                c += L
    for i in out["instances"]:
        if i["asset"] in BOUNDS:
            got.append({"name": "lay#" + i["key"], "x": i["at"][0], "y": i["at"][1], "z": level[i["surface"]],
                        "face": round(i["face"] * 1000), "bounds": BOUNDS[i["asset"]]})
    return got


def laid(attrs=None, **kw):
    a = dict(ATTRS)
    a.update(attrs or {})
    out = L["plan"]("hall", HALL, a, 10.0, None, KIT, **kw)
    return out, members(out)


class Doorways(unittest.TestCase):
    def test_the_laid_door_is_clear(self):
        """lay's rule 6 leaves the 1 m door 1.2 m between the bays (1 m, the 0.1 m margin, 0.1 m
        to close the stretch with fillers); measured from the placed pieces' footprints."""
        out, pieces = laid()
        (d,) = [f for f in house() if f["kind"] == "doorway"]
        w, (lo, hi) = C["clear_width"](d["path"], pieces, d["attrs"]["sill_z"])
        self.assertAlmostEqual(w, 1200, delta=2)
        self.assertEqual((lo, hi), ("lay#hall_s0_w1_1#r0", "lay#hall_s0_w1_0f#r0"))   # left of travel first
        # Counted in plan alone, the storey above's wall over the doorway would close it.
        self.assertEqual(C["clear_width"](d["path"], pieces)[0], 0.0)
        self.assertNotIn("door_narrow", [r[1] for r in C["judge"](house(), MEETINGS, [], P, pieces)])

    def test_a_narrow_door_names_the_doorway(self):
        """A 0.6 m door is left 0.7 m (0.8 m to close): under `inner_door` (0.9 m), named."""
        out, pieces = laid({"doors": [{"storey": 0, "t": 0.4, "width": 0.6}]})
        feats = house({"doors": [{"storey": 0, "t": 0.4, "width": 0.6}]})
        recs = [r for r in C["judge"](feats, MEETINGS, [], P, pieces) if r[1] == "door_narrow"]
        self.assertEqual([(r[0], r[2]) for r in recs], [("warning", "lay#hall_d0")])
        self.assertLess(recs[0][5]["clear"], 0.9)
        self.assertEqual(recs[0][6], (32.0, 24.0))
        self.assertEqual(C["address"]("door_narrow", recs[0][2]), "feature.lay#hall_d0")

    def test_a_piece_in_the_doorway_is_no_way_through(self):
        """A bay stood in the doorway (the station's front door, before the fix): clear 0."""
        out, pieces = laid()
        pieces = pieces + [{"name": "stray", "x": 31850, "y": 24000, "z": 10.3, "face": 90000,
                            "bounds": BOUNDS["bay"]}]
        recs = [r for r in C["judge"](house(), MEETINGS, [], P, pieces) if r[1] == "door_narrow"]
        self.assertEqual(recs[0][5]["clear"], 0.0)

    def test_no_pieces_measures_nothing(self):
        recs = C["judge"](house(), MEETINGS, [], P)
        self.assertEqual(recs[-1][5]["doorways"], 0)

    def test_footprint_turns_with_face(self):
        """A 1 m by 0.3 m bay facing east (90 degrees) is 0.3 m along x and 1 m along y."""
        fp = C["footprint"]({"x": 0, "y": 0, "face": 90000, "bounds": BOUNDS["bay"]})
        xs, ys = [p[0] for p in fp], [p[1] for p in fp]
        self.assertAlmostEqual(max(xs) - min(xs), 300)
        self.assertAlmostEqual(max(ys) - min(ys), 1000)


class Profile(unittest.TestCase):
    def test_a_steep_stair(self):
        feats = house({"stair": {"t": 0.025, "width": 1, "run": 3}})
        recs = [r for r in C["judge"](feats, MEETINGS, [], P) if r[1] == "stair"]
        self.assertEqual(len(recs), 1)
        self.assertEqual(recs[0][5], {"rise": 3.0, "length": 3.0, "risers": 16, "going": 0.188})

    def test_turn_is_judged_not_reported_twice(self):
        m = MEETINGS + [row("walk.end", "hut", 50000, 50000, turn=90000), row("lane.end", "hut2", 0, 0, turn=10000)]
        self.assertEqual(codes(C["judge"](house(), m, [], P)), [("turn", "walk.end")])

    def test_kit_pitch(self):
        """The college's bay: 6.44 m wide at a 6 m pitch."""
        kit = [{"id": "bay", "bounds": [-3220, -400, 0, 3220, 400, 9000], "params": {"pitch": 6}},
               {"id": "end", "bounds": [-3000, -400, 0, 3000, 400, 9000], "params": {"pitch": 6}},
               {"id": "rock", "bounds": [-1000, -1000, 0, 1000, 1000, 1000], "params": {}}]
        self.assertEqual(codes(C["judge"]([], [], kit, P)), [("kit_pitch", "bay")])

    PLAIN = [{"id": "wall", "taxon": "k", "tags": ["role:wall"], "bounds": [-1500, -150, 0, 1500, 150, 3000], "params": {"pitch": 3}},
             {"id": "half", "taxon": "k", "tags": ["role:filler"], "bounds": [-750, -150, 0, 750, 150, 3000], "params": {"pitch": 1.5}},
             {"id": "corner", "taxon": "k", "tags": ["role:corner"], "bounds": [-150, -150, 0, 150, 150, 3000], "params": {}},
             {"id": "corner_in", "taxon": "k", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 3000],
              "params": {"reach": 1.2}}]

    def test_kit_corner(self):
        """A post is as thick as the wall; an inside corner spans its reach along each wall from
        the notch it is placed by. A post re-exported with a 0.6 m stub is 0.9 by 0.3 m and is
        named; so is an inside corner with no reach said, or left on the default pivot."""
        self.assertEqual(codes(C["judge"]([], [], self.PLAIN, P)), [])
        stub = [dict(r, bounds=[-450, -150, 0, 450, 150, 3000]) if r["id"] == "corner" else r for r in self.PLAIN]
        (rec,) = [r for r in C["judge"]([], [], stub, P) if r[1] == "kit_corner"]
        self.assertEqual((rec[1], rec[2]), ("kit_corner", "corner"))
        self.assertEqual(rec[5], {"asset": "corner", "plan": [0.9, 0.3], "reach": 0.3, "square": 0.3})
        short = [dict(r, params={}) if r["id"] == "corner_in" else r for r in self.PLAIN]   # no reach said
        self.assertIn(("kit_corner", "corner_in"), codes(C["judge"]([], [], short, P)))
        centred = [dict(r, bounds=[-750, -750, 0, 750, 750, 3000]) if r["id"] == "corner_in" else r for r in self.PLAIN]
        self.assertEqual(codes(C["judge"]([], [], centred, P)), [("kit_corner", "corner_in")])

    def test_kit_height(self):
        """A bay 3.3 m high stands 0.3 m into the storey above on a 3 m storey; one storey alone
        has nothing above it, and a kit no building stacks is not judged."""
        tall = [dict(r, bounds=r["bounds"][:5] + [3300]) if r["id"] == "wall" else r for r in self.PLAIN]

        def building(storeys, kit="k"):
            return {"name": "hall", "kind": "building", "ring": HALL, "path": HALL,
                    "attrs": {"kit": kit, "storey_h": 3.0, "storeys": storeys}}
        (rec,) = [r for r in C["judge"]([building(2)], [], tall, P) if r[1] == "kit_height"]
        self.assertEqual((rec[1], rec[2]), ("kit_height", "wall"))
        self.assertEqual(rec[5], {"asset": "wall", "height": 3.3, "storey_h": 3.0, "buildings": ["hall"]})
        self.assertEqual(codes(C["judge"]([building(1)], [], tall, P)), [("kit_uneven", "wall")])
        self.assertEqual(codes(C["judge"]([building(2, "other")], [], tall, P)), [("kit_uneven", "wall")])
        self.assertEqual(codes(C["judge"]([building(2)], [], self.PLAIN, P)), [])

    def test_the_kit_against_itself(self):
        """The plain kit passes. Without its inside corner an L cannot close, and the check says
        what reach would. A 3.2 m bay beside a 1.5 m half bay shares no module; a 0.1 m slip in a
        0.3 m wall is a patch; a bay 3.3 m high among 3 m pieces is uneven."""
        kit = self.PLAIN
        half = next(r for r in kit if r["id"] == "half")
        self.assertEqual(codes(C["judge"]([], [], kit, P)), [])
        posts = [r for r in kit if r["id"] != "corner_in"]
        ell = [(0, 0), (9600, 0), (9600, 5100), (5100, 5100), (5100, 9600), (0, 9600)]

        def building(ring):
            return {"name": "annex", "kind": "building", "ring": ring, "path": ring,
                    "attrs": {"kit": "k", "storey_h": 3.0, "storeys": 1}}
        self.assertEqual(codes(C["judge"]([building(HALL)], [], posts, P)), [])     # a rectangle is not told
        for ring in (ell, ell[::-1]):                                                # however it winds
            (rec,) = [r for r in C["judge"]([building(ring)], [], posts, P) if r[0] != "info"]
            self.assertEqual((rec[1], rec[2]), ("kit_ell", "corner"))
            self.assertEqual(rec[5], {"taxon": "k", "outside": 0.3, "inside": 0.0, "module": 1.5, "reach": 1.2,
                                      "buildings": ["annex"]})
        self.assertEqual(codes(C["judge"]([building(ell)], [], kit, P)), [])
        wide = [dict(r, bounds=[-1600, -150, 0, 1600, 150, 3000], params={"pitch": 3.2}) if r["id"] == "wall" else r
                for r in kit]
        self.assertEqual(codes(C["judge"]([], [], wide, P)), [("kit_module", "wall")])
        slip = [dict(half, bounds=[-50, -150, 0, 50, 150, 3000], params={"pitch": 0.1}) if r["id"] == "half" else r
                for r in kit]
        self.assertIn(("kit_slip", "half"), codes(C["judge"]([], [], slip, P)))
        tall = [dict(r, bounds=r["bounds"][:5] + [3300]) if r["id"] == "wall" else r for r in kit]
        (rec,) = [r for r in C["judge"]([], [], tall, P) if r[0] != "info"]
        self.assertEqual((rec[1], rec[5]["measure"]), ("kit_uneven", "high"))

    def test_a_param_that_is_no_length_is_left_to_lay(self):
        """`reach = "wide"` or `pitch = true` raises nothing here: `lay` says it when it reads the
        kit, and this check reads the piece as if it said none."""
        for key, bad in (("reach", "wide"), ("reach", True), ("pitch", "3"), ("pitch", -2)):
            kit = [dict(r, params=dict(r["params"], **{key: bad})) for r in self.PLAIN]
            C["judge"]([], [], kit, P)

    def test_params(self):
        for bad in ({"riser": 1}, {"going_min": 0}, {"kits": "a"}):
            with self.assertRaises(C["ParamError"]):
                C["params_of"](bad)


if __name__ == "__main__":
    unittest.main()
