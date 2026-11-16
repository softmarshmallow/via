"""Conformance-style cases for reach.py through the engine: small worlds built by `via`, the lint run
as a plain command, its records read from `--diagnostics`. Each case cites the docstring row it
checks. Skipped unless VIA_BIN names a `via` binary; the SDK's interpreter is VIA_PYTHON (default:
this one, which needs NumPy).

    VIA_BIN=target/debug/via PYTHONPATH=sdk/python python3 -B -m unittest lib/review/test_reach_world.py

Every store, config and build goes under one temporary folder, removed at the end.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.dirname(HERE)
VIA = os.environ.get("VIA_BIN")

HEAD = """name    = "reach_case"
extent  = [0, 0, 40, 40]
resolution = 1
pack    = "starter"

[field.height]
role = "surface"
base = "{height}"

[field.water]
tags = ["water_level"]
base = "{water}"

[feature.a]
at = [5, 20]

[feature.b]
at = [35, 20]

[instance.hut]
asset = "house_low"
at    = [30, 10]
face  = 90

[step.reach]
when    = "lint"
worker  = true
command = ["python", "-m", "viasdk", "lib:review/reach.py"]

[step.reach.params]
reach = [{{ profile = "walker", from = "a", to = ["b", "hut.door"] }}]

[step.reach.params.profiles.walker]
{profile}
"""

WALKER = "grade = 0.5\nstep = 0.3\nwidth = 0.6\nwade = 0.3\nreach = 2\n"

# A beck 2 m down over x = 18..22 under 1 m of water, and a 4 m deck at 10 over it from x = 15 to
# 25, owned by a `span` (a kind of a scratch library, `span`, written by the test: kinds live in a
# library). `lanes` writes the two lanes that meet the deck's ends (surfaces.md 3.3).
SPAN = """[kind.span]
authority = "anchored"

[kind.span.surface.deck]
base  = "= 10"
bound = "= 2"
"""

DECK = """name    = "reach_deck"
extent  = [0, 0, 40, 40]
resolution = 1
pack    = "starter"
include = ["span"]

[field.height]
role = "surface"
base = "= 10 - 2 * smoothstep(17.4, 17.6, x) * (1 - smoothstep(22.4, 22.6, x))"

[field.water]
tags = ["water_level"]
base = "= max(f('height'), 9)"

[feature.a]
at = [5, 20]

[feature.b]
at = [35, 20]

[feature.bridge]
kind = "span"
path = [[15, 20], [25, 20]]
{lanes}
[step.reach]
when    = "lint"
{worker}command = ["python", "-m", "viasdk", "lib:review/reach.py"]

[step.reach.params]
reach = [{{ profile = "walker", from = "a", to = ["b", "bridge.start", "bridge.end"] }}]

[step.reach.params.profiles.walker]
{profile}
"""

LANES = """
[feature.lane_west]
starts_at = "a"
ends_at   = "bridge.start"
path      = [[10, 20]]

[feature.lane_east]
starts_at = "bridge.end"
ends_at   = "b"
path      = [[30, 20]]
"""


@unittest.skipUnless(VIA, "VIA_BIN not set: the engine cases need a `via` binary")
class ReachWorld(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="via-reach-")
        lib = os.path.join(cls.tmp, "lib")          # the real libraries, and `span`
        os.makedirs(os.path.join(lib, "span"))
        for name in os.listdir(LIB):
            if os.path.isdir(os.path.join(LIB, name)):
                os.symlink(os.path.join(LIB, name), os.path.join(lib, name))
        with open(os.path.join(lib, "span", "kinds.toml"), "w") as f:
            f.write(SPAN)
        cls.env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(cls.tmp, "cfg"), VIA_LIB=lib,
                       TMPDIR=cls.tmp)
        py = os.environ.get("VIA_PYTHON", sys.executable)
        subprocess.run([VIA, "config", "program", "python", py], env=cls.env, check=True,
                       capture_output=True)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    def build(self, name, height="= 10", water="= f('height')", profile=WALKER, text=None):
        d = os.path.join(self.tmp, name)
        os.makedirs(d)
        world = os.path.join(d, "world.toml")
        with open(world, "w") as f:
            f.write(text or HEAD.format(height=height, water=water, profile=profile))
        subprocess.run([VIA, "trust", world, "--yes"], env=self.env, check=True, capture_output=True)
        diag = os.path.join(d, "diag.json")
        got = subprocess.run([VIA, "build", world, "--store", os.path.join(d, "store"), "--out",
                              os.path.join(d, "out"), "--diagnostics", diag], env=self.env,
                             capture_output=True, text=True)
        with open(diag) as f:
            d = json.load(f)
        recs = d["diagnostics"] if isinstance(d, dict) else d
        return got.returncode, [r for r in recs if r["code"].startswith("step.reach.")
                                or r["code"] == "protocol.failed"]

    def deck(self, name, lanes=LANES, worker="worker  = true\n"):
        return self.build(name, text=DECK.format(lanes=lanes, worker=worker, profile=WALKER))

    def codes(self, recs):
        return sorted(r["code"] for r in recs)

    def test_flat_holds_and_the_door_is_found(self):
        # rows `walked`; a place `<instance>.<mount>` (house_low's door at (0, 3), face 90: (33, 10))
        code, recs = self.build("flat")
        self.assertEqual(code, 0)
        self.assertEqual(self.codes(recs), ["step.reach.walked"])
        self.assertEqual(recs[0]["data"]["held"], 2)

    def test_cliff_is_steep(self):
        # row `unreached`: a 2 m face between posts 19 and 20
        code, recs = self.build("cliff", height="= 10 + 2 * smoothstep(19.4, 19.6, x)")
        un = [r for r in recs if r["code"] == "step.reach.unreached"]
        self.assertEqual(len(un), 2, recs)
        b = next(r for r in un if r["data"]["to"] == "b")
        self.assertEqual((b["data"]["why"], b["position"]), ("steep", [19.0, 20.0]))

    def test_pond_is_deep(self):
        # row `unreached`, why `water`: 0.5 m of water over x 18..22
        code, recs = self.build(
            "pond", water="= f('height') + 0.5 * smoothstep(17.4, 17.6, x) * (1 - smoothstep(22.4, 22.6, x))")
        b = next(r for r in recs if r["code"] == "step.reach.unreached" and r["data"]["to"] == "b")
        self.assertEqual((b["data"]["why"], b["data"]["value"]), ("water", 0.5))

    def test_missing_profile_key_is_an_error(self):
        # row `profile`: no default is assumed
        code, recs = self.build("bare", profile="grade = 0.5\n")
        self.assertNotEqual(code, 0)
        self.assertEqual(self.codes(recs), ["step.reach.profile"])

    def test_deck_is_walked_through_its_meetings(self):
        # surfaces.md 4.4 through the engine: `surfaces_at` gives the deck's posts, `meetings` the
        # lanes' ends at the deck's; the walk crosses the beck on the deck and reaches both ends
        code, recs = self.deck("deck")
        self.assertEqual(self.codes(recs), ["step.reach.walked"], recs)
        d = recs[0]["data"]
        self.assertEqual((d["held"], d["pairs"], d["surfaces"], d["joins"]), (3, 3, 1, 2))

    def test_deck_with_no_meeting_is_an_island(self):
        # no lane meets the deck: its bound's edge is a wall (item 3), the beck cuts a from b, and
        # the deck's own ends are not reached though the bank under them is level with them
        code, recs = self.deck("island", lanes="")
        un = sorted(r["data"]["to"] for r in recs if r["code"] == "step.reach.unreached")
        self.assertEqual(un, ["b", "bridge.end", "bridge.start"], recs)

    def test_plain_command_walks_the_terrain_only(self):
        # row `terrain_only`: a plain command asks the engine nothing
        code, recs = self.deck("plain", worker="")
        self.assertIn("step.reach.terrain_only", self.codes(recs))
        self.assertIn("step.reach.unreached", self.codes(recs))


if __name__ == "__main__":
    unittest.main()
