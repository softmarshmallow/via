"""Tests for check.py: reach between surfaces and rooms, stairs, turns, the kit's pitch.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The meetings are surfaces.md 6.4's four rows, as `surface.meetings` gives them; the features are
what lay.py returns for that house (names as the build gives them, `lay#<key>`).
"""

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
                      "attrs": {}})
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

    def test_params(self):
        for bad in ({"riser": 1}, {"going_min": 0}, {"kits": "a"}):
            with self.assertRaises(C["ParamError"]):
                C["params_of"](bad)


if __name__ == "__main__":
    unittest.main()
