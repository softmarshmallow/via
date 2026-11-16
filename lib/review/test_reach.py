"""Tests for reach.py, each from a row of its docstring, on small made-up worlds with a stand-in
for the SDK's ``step`` (it answers ``fields``, ``features``, ``instances``, ``pack``,
``surfaces_at`` and ``meetings`` as protocol.md 3.3 and 4.3 shape them, and records what is said).

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/review -p 'test_*.py'
"""

import json
import os
import runpy
import unittest

R = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), "reach.py"))

N = 41  # posts 1 m apart over 0..40 m

WALKER = {"grade": 0.5, "step": 0.3, "width": 0.6, "wade": 0.3, "reach": 2}


class Cols(dict):
    pass


def feature_list(feats):
    cols = Cols(name=[], kind=[], geometry=[], pieces=[0], rings=[0], ring_starts=[0], coords=[])
    for name, kind, geom, pieces in feats:
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


PACK = {
    # a 4 by 2 m house, door on its front (+y) wall's middle
    "hut": {"taxon": "building/house", "footprint": [[-2000, -1000], [2000, -1000], [2000, 1000], [-2000, 1000]],
            "solidity": "solid", "mounts": {"door": {"at": [0, 1000], "face": 0}}},
    # a wall 1 m thick and 40 m long
    "wall": {"taxon": "prop/wall", "footprint": [[-500, -20000], [500, -20000], [500, 20000], [-500, 20000]],
             "solidity": "solid", "mounts": {}},
    "mist": {"taxon": "prop/mist", "footprint": [[-500, -20000], [500, -20000], [500, 20000], [-500, 20000]],
             "solidity": "none", "mounts": {}},
}


class FakeStep:
    """`kinds` maps a kind surface's place name to (x0, y0, x1, y1, z): a level rectangle, metres,
    edges in; `meets` is a list of meeting rows (surfaces.md 2.4) in metres and millimetres."""

    def __init__(self, height, water=None, feats=(), insts=(), params=None, kinds=None, meets=()):
        self.id = "reach"
        self._kinds = dict(kinds or {})
        self._meets = list(meets)
        self.grid = {"extent_mm": [0, 0, 40000, 40000], "resolution_mm": 1000, "nx": N, "ny": N}
        self._f = {"role:surface": height}
        if water is not None:
            self._f["tag:water_level"] = water
        self._feats = list(feats)
        self._insts = list(insts)
        self.params = params or {}
        self.said = []

    def fields(self, names):
        if names[0] not in self._f:
            raise LookupError(f"{names[0]} selects no field")
        return [self._f[n] for n in names]

    def features(self, select):
        return feature_list(self._feats)

    def instances(self, select="*"):
        keys = ("name", "asset", "by", "x", "y", "face", "scale")
        cols = Cols({k: [r.get(k, {"by": "", "face": 0, "scale": 1.0}.get(k)) for r in self._insts] for k in keys})
        cols["z"] = [r["z"] if "z" in r else self._terrain(r["x"], r["y"]) for r in self._insts]
        return cols

    def _terrain(self, x, y):
        h = self._f["role:surface"]
        return h[min(max(round(y / 1000), 0), N - 1)][min(max(round(x / 1000), 0), N - 1)]

    def surfaces_at(self, xs, ys):
        rows = []
        for k, (x, y) in enumerate(zip(xs, ys)):
            here = [("height", self._terrain(x, y))]
            for name, (x0, y0, x1, y1, z) in self._kinds.items():
                if x0 * 1000 <= x <= x1 * 1000 and y0 * 1000 <= y <= y1 * 1000:
                    here.append((name, z))
            rows += [(k, n, z) for n, z in sorted(here, key=lambda r: (r[1], r[0]))]
        return Cols(point=[r[0] for r in rows], surface=[r[1] for r in rows], z=[r[2] for r in rows])

    def meetings(self):
        keys = ("a", "b", "x", "y", "gap", "z_a", "z_b", "connector_a", "connector_b")
        cols = Cols({k: [m.get(k) for m in self._meets] for k in keys})
        for k in ("connector_a", "connector_b"):
            cols[k + "?"] = [1 if v else 0 for v in cols[k]]
            cols[k] = [v or "" for v in cols[k]]
        return cols

    def pack(self, select="*", columns=()):
        ids = sorted(PACK)
        return Cols(id=ids, taxon=[PACK[a]["taxon"] for a in ids],
                    footprint=[json.dumps(PACK[a]["footprint"]) for a in ids],
                    solidity=[PACK[a]["solidity"] for a in ids],
                    mounts=[json.dumps(PACK[a]["mounts"]) for a in ids])

    def _say(self, sev, code, subject, message, hint, **kw):
        self.said.append(dict(kw, severity=sev, code=code, subject=subject, message=message))

    def error(self, *a, **kw):
        self._say("error", *a, **kw)

    def warning(self, *a, **kw):
        self._say("warning", *a, **kw)

    def info(self, *a, **kw):
        self._say("info", *a, **kw)

    def codes(self):
        return [r["code"] for r in self.said]

    def one(self, code):
        got = [r for r in self.said if r["code"] == code]
        assert len(got) == 1, (code, self.said)
        return got[0]


def flat(z=10.0):
    return [[z] * N for _ in range(N)]


def pt(name, x, y):
    return (name, "", 0, [[[(x * 1000, y * 1000)]]])


def run(height, water=None, feats=(), insts=(), profile=None, kinds=None, meets=(), **params):
    params.setdefault("profiles", {"walker": dict(WALKER, **(profile or {}))})
    s = FakeStep(height, water, [pt("a", 5, 20), pt("b", 35, 20)] + list(feats), insts, params,
                 kinds, meets)
    R["lint"](s)
    return s


def a_to_b(**kw):
    return [dict({"profile": "walker", "from": "a", "to": ["b"]}, **kw)]


class Reach(unittest.TestCase):
    def test_flat_world_holds(self):
        s = run(flat(), reach=a_to_b())
        self.assertEqual(s.codes(), ["walked"])
        self.assertEqual(s.one("walked")["data"], {"posts": N * N, "open": N * N, "pieces": 1,
                                                   "held": 1, "pairs": 1, "surfaces": 0,
                                                   "joins": 0})

    def test_cliff_stops_walk_and_says_steep(self):
        h = [[10.0 if i < 20 else 12.0 for i in range(N)] for _ in range(N)]  # a 2 m face at x = 20
        s = run(h, reach=a_to_b())
        r = s.one("unreached")
        self.assertEqual(r["severity"], "warning")
        self.assertEqual(r["data"]["why"], "steep")
        self.assertEqual(r["data"]["value"], 2.0)
        self.assertEqual(r["position"], [19.0, 20.0])  # the last post walked, nearest b
        self.assertEqual(r["data"]["gap"], 16.0)
        self.assertEqual(s.one("walked")["data"]["held"], 0)

    def test_grade_and_step_both_let_a_rise_through(self):
        ramp = [[10.0 + 0.4 * i for i in range(N)] for _ in range(N)]     # 0.4 per metre
        self.assertEqual(run(ramp, reach=a_to_b()).codes(), ["walked"])
        # a diagonal climbs 0.4 / sqrt 2 = 0.28 per metre, as a switchback does
        self.assertEqual(run(ramp, reach=a_to_b(), profile={"grade": 0.3}).codes(), ["walked"])
        self.assertEqual(run(ramp, reach=a_to_b(), profile={"grade": 0.25}).one("unreached")
                         ["data"]["why"], "steep")
        self.assertEqual(run(ramp, reach=a_to_b(), profile={"grade": 0.1, "step": 0.45}).codes(),
                         ["walked"])

    def test_water_deeper_than_wade(self):
        h = flat()
        w = [[10.5 if 18 <= i <= 22 else 10.0 for i in range(N)] for _ in range(N)]  # 0.5 m deep
        r = run(h, w, reach=a_to_b()).one("unreached")
        self.assertEqual((r["data"]["why"], r["data"]["value"]), ("water", 0.5))
        self.assertEqual(r["position"], [17.0, 20.0])
        self.assertEqual(run(h, w, reach=a_to_b(), profile={"wade": 0.5}).codes(), ["walked"])

    def test_solid_instance_blocks_and_is_named(self):
        insts = [{"name": "rampart", "asset": "wall", "x": 20000, "y": 20000}]
        r = run(flat(), insts=insts, reach=a_to_b()).one("unreached")
        self.assertEqual((r["data"]["why"], r["data"]["by"]), ("instance", "rampart"))
        mist = [{"name": "fog", "asset": "mist", "x": 20000, "y": 20000}]
        self.assertEqual(run(flat(), insts=mist, reach=a_to_b()).codes(), ["walked"])

    def test_width_shuts_posts_near_an_obstacle(self):
        insts = [{"name": "w1", "asset": "wall", "x": 20000, "y": 0},          # y -20..20
                 {"name": "w2", "asset": "wall", "x": 20000, "y": 44000}]      # y 24..64
        # a 3 m gap between the walls (posts y = 21, 22, 23): a 2 m body fits, a 5 m one does not
        self.assertEqual(run(flat(), insts=insts, reach=a_to_b(), profile={"width": 2}).codes(),
                         ["walked"])
        r = run(flat(), insts=insts, reach=a_to_b(), profile={"width": 5}).one("unreached")
        self.assertEqual(r["data"]["why"], "narrow")

    def test_not_sealed_names_the_routes_narrowest_point(self):
        # A sealed pair joined through the 3 m gap between two walls at x = 20 (posts y = 21 to
        # 23): `data.narrowest` is in the gap, 1 m or less from a wall (TODO, "`not_sealed` says no
        # place"; the monastery's LOG found the way in by a throwaway walk).
        insts = [{"name": "w1", "asset": "wall", "x": 20000, "y": 0},
                 {"name": "w2", "asset": "wall", "x": 20000, "y": 44000}]
        r = run(flat(), insts=insts, sealed=a_to_b(), profile={"width": 0.6}).one("not_sealed")
        n = r["data"]["narrowest"]
        self.assertEqual(r["position"], [35.0, 20.0])               # still at the `to` place
        self.assertTrue(19.0 <= n["at"][0] <= 21.0 and 21.0 <= n["at"][1] <= 23.0, n)
        self.assertLessEqual(n["clear"], 1.0)
        self.assertIn(n["by"], ("w1", "w2"))
        self.assertIn("narrowest at", r["message"])

    def test_door_mounts_resolve_by_group_and_face(self):
        # hut at (30, 10) facing east (90): its door at asset (0, 1) lies at world (31, 10)
        insts = [{"name": "hut1", "asset": "hut", "x": 30000, "y": 10000, "face": 90000}]
        p = R["Places"](R["Grid"]({"extent_mm": [0, 0, 40000, 40000], "resolution_mm": 1000,
                                   "nx": N, "ny": N}), [], R["_instances"](FakeStep(flat(), insts=insts)),
                        R["_pack"](FakeStep(flat())), None)
        self.assertEqual(p.resolve({"mount": "door", "assets": ["@building/house"]}),
                         [("hut1.door", (31000, 10000), "")])
        self.assertEqual(p.resolve("hut1.door"), [("hut1.door", (31000, 10000), "")])
        s = run(flat(), insts=insts,
                reach=[{"profile": "walker", "from": "a", "to": [{"mount": "door", "assets": ["hut"]}]}])
        self.assertEqual(s.codes(), ["walked"])  # the door's post is in the footprint; reach finds 32

    def test_stranded_place(self):
        insts = [{"name": "hut1", "asset": "hut", "x": 35000, "y": 20000}]  # b inside the hut
        s = run(flat(), insts=insts, reach=a_to_b(), profile={"reach": 0.5})
        r = s.one("stranded")
        self.assertEqual((r["data"]["why"], r["data"]["by"]), ("instance", "hut1"))
        self.assertNotIn("unreached", s.codes())

    def test_sealed_island(self):
        h = flat()
        w = [[10.0 if abs(i - 35) <= 3 and abs(j - 20) <= 3 else 11.0 for i in range(N)]
             for j in range(N)]  # a 7 m island round b in 1 m of water
        w = [[max(h[j][i], w[j][i]) for i in range(N)] for j in range(N)]
        for j in range(N):  # a dry shore round a
            for i in range(10):
                w[j][i] = 10.0
        s = run(h, w, sealed=a_to_b())
        self.assertEqual(s.codes(), ["walked"])
        self.assertEqual(s.one("walked")["data"]["held"], 1)
        sunk = [[max(v, 12.0) if abs(i - 35) <= 3 and abs(j - 20) <= 3 else v for i, v in enumerate(row)]
                for j, row in enumerate(w)]  # the island under 2 m: nowhere to stand, still sealed
        self.assertEqual(run(h, sunk, sealed=a_to_b()).one("walked")["data"]["held"], 1)
        r = run(h, w, sealed=a_to_b(), profile={"wade": 1.5}).one("not_sealed")
        self.assertEqual(r["position"], [35.0, 20.0])

    def test_link_joins_across(self):
        h = [[10.0 if i < 20 else 14.0 for i in range(N)] for _ in range(N)]
        feats = [pt("foot", 19, 20), pt("top", 21, 20)]
        s = run(h, feats=feats, reach=a_to_b(),
                links=[{"profile": "walker", "a": "foot", "b": "top", "reason": "a ladder"}])
        self.assertEqual(s.codes(), ["walked"])

    def test_profile_without_a_key_is_an_error(self):
        s = run(flat(), reach=a_to_b(), profiles={"walker": {"grade": 0.5}})
        r = s.one("profile")
        self.assertEqual(r["severity"], "error")
        self.assertEqual(r["data"]["missing"], ["step", "width", "wade", "reach"])
        self.assertEqual(s.codes(), ["profile"])

    def test_unknown_place_is_an_error(self):
        s = run(flat(), reach=[{"profile": "walker", "from": "a", "to": ["nowhere", "b"]}])
        self.assertEqual(s.one("unknown_place")["severity"], "error")
        self.assertEqual(s.one("walked")["data"]["held"], 1)

    def test_waiver_turns_a_record_to_info_and_unused_is_said(self):
        h = [[10.0 if i < 20 else 12.0 for i in range(N)] for _ in range(N)]
        s = run(h, reach=a_to_b(), waive=[{"to": "b", "reason": "reached by boat"},
                                          {"to": "a", "reason": "stale"}])
        r = s.one("waived")
        self.assertEqual((r["severity"], r["data"]["waived"]), ("info", "unreached"))
        self.assertEqual(s.one("waiver_unused")["subject"], "step.reach")
        self.assertEqual(r["subject"], "b")
        self.assertNotIn("unreached", s.codes())

    def test_no_ground_is_unmeasured(self):
        s = FakeStep(flat(), params={"profiles": {"walker": WALKER}, "reach": a_to_b()})
        s._f = {}
        R["lint"](s)
        self.assertEqual(s.codes(), ["unmeasured"])

    def test_same_records_twice(self):
        h = [[10.0 + (3.0 if (i * 7 + j * 3) % 11 == 0 else 0.0) for i in range(N)] for j in range(N)]
        a = run(h, reach=a_to_b()).said
        b = run(h, reach=a_to_b()).said
        self.assertEqual(json.dumps(a, sort_keys=True), json.dumps(b, sort_keys=True))

    def test_first_record_of_a_code_explains(self):
        h = [[10.0 if i < 20 else 12.0 for i in range(N)] for _ in range(N)]
        feats = [pt("c", 36, 25)]
        s = run(h, feats=feats, reach=[{"profile": "walker", "from": "a", "to": ["b", "c"]}])
        un = [r for r in s.said if r["code"] == "unreached"]
        self.assertEqual(len(un), 2)
        self.assertIn("explain", un[0])
        self.assertNotIn("explain", un[1])


def river():
    """A beck over x = 18..22 the whole height of the world, its bed at 8 under 1 m of water, in
    flat ground at 10."""
    h = [[8.0 if 18 <= i <= 22 else 10.0 for i in range(N)] for _ in range(N)]
    return h, [[9.0 if 18 <= i <= 22 else 10.0 for i in range(N)] for _ in range(N)]


DECK = {"bridge.deck": (15, 18, 25, 22, 10.0)}            # x 15..25, y 18..22, level with the banks


def ends(gap_east=0, connector=None):
    """The two meetings of a crossing: the lanes' ends at the deck's ends (surfaces.md 6.3)."""
    out = []
    for a, b, x, gap in (("bridge.start", "lane_west.end", 15000, 0),
                         ("bridge.end", "lane_east.start", 25000, gap_east)):
        out.append({"a": a, "b": b, "x": x, "y": 20000, "gap": gap, "z_a": 10.0,
                    "z_b": 10.0 + gap / 1000.0, "connector_a": connector, "connector_b": None})
    return out


def crossing():
    """The crossing's path, so `bridge.start`/`.end` are places on its one surface."""
    return ("bridge", "crossing", 1, [[[(15000, 20000), (25000, 20000)]]])


class KindSurfaces(unittest.TestCase):
    """surfaces.md 4.4, run by the walk: one node per (surface, post); a surface's own posts joined
    as the terrain's; two surfaces only at a meeting that holds."""

    def test_walks_onto_a_deck_through_a_meeting_that_holds(self):
        h, w = river()
        self.assertEqual(run(h, w, reach=a_to_b()).one("unreached")["data"]["why"], "water")
        s = run(h, w, reach=a_to_b(), kinds=DECK, meets=ends())
        self.assertEqual(s.codes(), ["walked"])
        d = s.one("walked")["data"]
        self.assertEqual((d["held"], d["surfaces"], d["joins"], d["posts"]), (1, 1, 2, N * N + 11 * 5))

    def test_a_deck_is_joined_only_at_its_meetings(self):
        # 4.4 item 3: a bound's edge is a wall; with no meeting the deck is an island over the beck
        h, w = river()
        r = run(h, w, reach=a_to_b(), kinds=DECK).one("unreached")
        self.assertEqual(r["data"]["why"], "water")
        # the deck's own end is a place on the deck: not reached from the lane without a meeting,
        # though the ground beneath it is dry and level with it
        s = run(h, w, feats=[crossing()], kinds=DECK,
                reach=[{"profile": "walker", "from": "a", "to": ["bridge.start"]}])
        self.assertEqual(s.one("unreached")["data"]["to"], "bridge.start")
        s = run(h, w, feats=[crossing()], kinds=DECK, meets=ends(),
                reach=[{"profile": "walker", "from": "a", "to": ["bridge.start", "bridge.end"]}])
        self.assertEqual(s.codes(), ["walked"])

    def test_a_meeting_that_does_not_hold_joins_nothing(self):
        h, w = river()
        r = run(h, w, reach=a_to_b(), kinds=DECK, meets=ends(gap_east=300)).one("unreached")
        self.assertEqual(r["data"].get("surface"), "bridge.deck")   # the walk ends on the deck
        self.assertEqual(r["data"]["why"], "edge")
        two = ends()
        two[1]["connector_a"], two[1]["connector_b"] = "stone", "timber"
        self.assertIn("unreached", run(h, w, reach=a_to_b(), kinds=DECK, meets=two).codes())

    def test_a_footprint_shuts_only_the_surface_it_stands_on(self):
        h, w = river()
        # a wall across the whole world at x = 20, standing in the bed (the terrain, z 8): the deck
        # passes over it
        bed = [{"name": "weir", "asset": "wall", "x": 20000, "y": 20000, "z": 8.0}]
        self.assertEqual(run(h, w, insts=bed, reach=a_to_b(), kinds=DECK, meets=ends()).codes(),
                         ["walked"])
        # the same wall standing on the deck (its z the deck's) shuts the deck
        on = [{"name": "gate", "asset": "wall", "x": 20000, "y": 20000, "z": 10.0}]
        r = run(h, w, insts=on, reach=a_to_b(), kinds=DECK, meets=ends()).one("unreached")
        self.assertEqual((r["data"]["why"], r["data"]["by"], r["data"]["surface"]),
                         ("instance", "gate", "bridge.deck"))

    def test_water_is_read_against_the_ground(self):
        # a deck post is wet only where water stands over the ground there and over the deck by
        # more than `wade`: a deck laid 1 m under the beck's surface is not walked
        h, w = river()
        sunk = {"bridge.deck": (15, 18, 25, 22, 8.0)}
        two = [dict(m, z_a=8.0, z_b=8.0) for m in ends()]
        hh = [[8.0 if 15 <= i <= 25 else 10.0 for i in range(N)] for _ in range(N)]
        ww = [[9.0 if 15 <= i <= 25 else 10.0 for i in range(N)] for _ in range(N)]
        r = run(hh, ww, reach=a_to_b(), kinds=sunk, meets=two, profile={"grade": 2.0}).one("unreached")
        self.assertEqual((r["data"]["why"], r["data"]["value"]), ("water", 1.0))
        # dry ground under a floor below the terrain's level (a cellar): the water field there is
        # the ground (lib/water's dry convention), so the floor is dry
        cellar = {"bridge.deck": (15, 18, 25, 22, 7.0)}
        d = run(flat(), flat(), reach=a_to_b(), kinds=cellar).one("walked")["data"]
        self.assertEqual((d["open"], d["posts"]), (N * N + 11 * 5, N * N + 11 * 5))

    def test_a_kind_surfaces_edge_narrows_as_a_wall(self):
        h, w = river()
        self.assertEqual(run(h, w, reach=a_to_b(), kinds=DECK, meets=ends(),
                             profile={"width": 2.5}).codes(), ["walked"])
        r = run(h, w, reach=a_to_b(), kinds=DECK, meets=ends(), profile={"width": 6.5})
        self.assertIn("unreached", r.codes())

    def test_a_point_stands_on_the_lowest_open_surface(self):
        # `mid` over the beck: the bed is under water, so it stands on the deck
        h, w = river()
        s = run(h, w, feats=[pt("mid", 20, 20)], kinds=DECK, meets=ends(),
                reach=[{"profile": "walker", "from": "a", "to": ["mid"]}])
        self.assertEqual(s.codes(), ["walked"])

    def test_a_plain_command_walks_the_terrain_only_and_says_so(self):
        h, w = river()
        s = FakeStep(h, w, [pt("a", 5, 20), pt("b", 35, 20)], (),
                     {"profiles": {"walker": WALKER}, "reach": a_to_b()}, DECK, ends())

        def refuse(xs, ys):
            from viasdk import NotAWorker
            raise NotAWorker("surface.surfaces_at needs a worker (worker = true)")
        s.surfaces_at = refuse
        R["lint"](s)
        self.assertEqual(s.codes(), ["terrain_only", "unreached", "walked"])
        self.assertEqual(s.one("terrain_only")["severity"], "info")


class Mounts(unittest.TestCase):
    def test_a_group_names_several_mounts(self):
        # a corner house's front door and its side door, both reached (quay, TODO "From M5 slice
        # 3, town kit"): `mount` as a list gives each instance's mounts in the order written
        PACK["corner"] = {"taxon": "building/house", "solidity": "solid",
                          "footprint": [[-3000, -3000], [3000, -3000], [3000, 3000], [-3000, 3000]],
                          "mounts": {"door": {"at": [0, 3000], "face": 0},
                                     "side": {"at": [3000, 0], "face": 90000}}}
        try:
            insts = [{"name": "c1", "asset": "corner", "x": 30000, "y": 10000}]
            g = R["Grid"]({"extent_mm": [0, 0, 40000, 40000], "resolution_mm": 1000, "nx": N, "ny": N})
            p = R["Places"](g, [], R["_instances"](FakeStep(flat(), insts=insts)),
                            R["_pack"](FakeStep(flat())), None)
            self.assertEqual(p.resolve({"mount": ["door", "side"], "assets": ["@building/house"]}),
                             [("c1.door", (30000, 13000), ""), ("c1.side", (33000, 10000), "")])
            s = run(flat(), insts=insts,
                    reach=[{"profile": "walker", "from": "a",
                            "to": [{"mount": ["door", "side"], "assets": ["corner"]}]}])
            self.assertEqual(s.one("walked")["data"]["held"], 2)
        finally:
            del PACK["corner"]


if __name__ == "__main__":
    unittest.main()
