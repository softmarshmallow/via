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
command = ["python", "-m", "viasdk", "lib:review/reach.py"]

[step.reach.params]
reach = [{{ profile = "walker", from = "a", to = ["b", "hut.door"] }}]

[step.reach.params.profiles.walker]
{profile}
"""

WALKER = "grade = 0.5\nstep = 0.3\nwidth = 0.6\nwade = 0.3\nreach = 2\n"


@unittest.skipUnless(VIA, "VIA_BIN not set: the engine cases need a `via` binary")
class ReachWorld(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="via-reach-")
        cls.env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(cls.tmp, "cfg"), VIA_LIB=LIB,
                       TMPDIR=cls.tmp)
        py = os.environ.get("VIA_PYTHON", sys.executable)
        subprocess.run([VIA, "config", "program", "python", py], env=cls.env, check=True,
                       capture_output=True)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    def build(self, name, height="= 10", water="= f('height')", profile=WALKER):
        d = os.path.join(self.tmp, name)
        os.makedirs(d)
        world = os.path.join(d, "world.toml")
        with open(world, "w") as f:
            f.write(HEAD.format(height=height, water=water, profile=profile))
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


if __name__ == "__main__":
    unittest.main()
