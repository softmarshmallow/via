"""Small-world cases for levels (lib/ways kinds.toml: `viaduct`, `ramp`, `interchange`'s `level`;
M5 slice 3, challenge 38), built by `via`. Skipped unless VIA_BIN names a `via` binary; the SDK's
interpreter is VIA_PYTHON (default: this one).

    VIA_BIN=target/release/via PYTHONPATH=sdk/python python3 -B -m unittest lib/ways/test_levels_world.py

The world (flat ground at 10 m): a road runs east along y = 100 in two pieces that meet at an
interchange (100, 100), whose plate is taken 2 m down; a freeway runs north along x = 100 in two
pieces that end at a viaduct's ends by name, the viaduct over the interchange, its middle `deck_z`
16; one loop ramp starts on the deck as a place and lands on the ground west of the freeway at a
point by name (a 270-degree loop of radius 35 m from (95, 140) to (60, 105)). Every store, config and build goes under one temporary folder, removed at the end.
"""

import json
import math
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.dirname(HERE)
VIA = os.environ.get("VIA_BIN")
PACK = os.path.join(LIB, "packs", "primitive.toml")

LOOP = ", ".join(f"[{60 + 35 * math.cos(math.radians(a)):.2f}, {140 + 35 * math.sin(math.radians(a)):.2f}]"
                 for a in range(0, 270, 15))

WORLD = f"""name    = "levels_case"
extent  = [0, 0, 200, 260]
resolution = 1
include = ["ways"]
pack    = "{PACK}"

[field.height]
role = "surface"
base = "= 10"

[feature.west_end]
at = [0, 100]
[feature.east_end]
at = [200, 100]
[feature.south_end]
at = [100, 0]
[feature.north_end]
at = [100, 260]

[feature.ix]
kind  = "interchange"
at    = [100, 100]
level = 8
reach = 25

[feature.road_w]
kind      = "road"
starts_at = "west_end"
ends_at   = "ix"
path      = []
[feature.road_e]
kind      = "road"
starts_at = "ix"
ends_at   = "east_end"
path      = []

[feature.fly_s]
at = [100, 20]
[feature.fly_n]
at = [100, 220]

[feature.fly]
kind      = "viaduct"
starts_at = "fly_s"
ends_at   = "fly_n"
path      = []
z_start   = 10
z_end     = 10
deck_z    = {{deck_z}}
width     = 12

[feature.free_s]
kind      = "road"
starts_at = "south_end"
ends_at   = "fly.start"
path      = []
[feature.free_n]
kind      = "road"
starts_at = "fly.end"
ends_at   = "north_end"
path      = []

[feature.foot]
at = [60, 105]

[feature.loop]
kind      = "ramp"
starts_at = "fly.deck"
ends_at   = "foot"
path      = [{LOOP}]
z0        = "= fly.deck_z"
z1        = 10

[network.grid]
from   = ["kind:road", "kind:viaduct"]
"""


def field(build, name):
    with open(os.path.join(build, "fields", f"{name}.npy"), "rb") as f:
        raw = f.read()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    ny, nx = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    v = struct.unpack(f"<{nx * ny}f", raw[10 + hlen:])
    return lambda i, j: v[j * nx + i]


@unittest.skipUnless(VIA, "VIA_BIN not set: the world cases need a `via` binary")
class LevelsWorld(unittest.TestCase):
    @classmethod
    def build_world(cls, name, deck_z=16):
        world = os.path.join(cls.tmp, f"{name}.toml")
        with open(world, "w") as f:
            f.write(WORLD.replace("{deck_z}", str(deck_z)))
        build = os.path.join(cls.tmp, name)
        run = subprocess.run([VIA, "build", world, "--out", build, "--store", os.path.join(cls.tmp, "store")],
                             env=cls.env, capture_output=True, text=True)
        return build, run

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="via-levels-")
        cls.env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(cls.tmp, "cfg"), VIA_LIB=LIB, TMPDIR=cls.tmp)
        subprocess.run([VIA, "config", "program", "python", os.environ.get("VIA_PYTHON", sys.executable)],
                       env=cls.env, check=True, capture_output=True)
        cls.build, cls.done = cls.build_world("build")

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    def meets(self, build=None):
        with open(os.path.join(build or self.build, "meets.json")) as f:
            return {(m["a"], m["b"]): m for m in json.load(f)["meets"]}

    def codes(self, build=None):
        with open(os.path.join(build or self.build, "diagnostics.json")) as f:
            return [d["code"] for d in json.load(f)["diagnostics"]]

    def test_builds_with_no_error_and_every_crossing_owned(self):
        """The viaduct over the road (one network) and the ramp from the deck (named onto it) own
        every crossing of lines: no error, no `geom.unowned_crossing`."""
        self.assertEqual(self.done.returncode, 0, self.done.stdout + self.done.stderr)
        codes = self.codes()
        self.assertNotIn("geom.unowned_crossing", codes)
        self.assertNotIn("authority.squeezed", codes)

    def test_every_level_meeting_is_at_gap_0(self):
        """The freeway meets each end of the viaduct at the ground's 10 m (heads hold it); the loop
        leaves the deck's level middle at `deck_z` and lands at its foot on the ground (its `foot`
        holds 10 m there): four meetings, each gap 0 mm."""
        rows = self.meets()
        for pair, z in ((("free_s.end", "fly.start"), 10), (("free_n.start", "fly.end"), 10),
                        (("loop.start", "fly.deck"), 16), (("loop.end", "foot"), 10)):
            self.assertIn(pair, rows, sorted(rows))
            self.assertEqual(rows[pair]["gap"], 0, rows[pair])
            self.assertAlmostEqual(rows[pair]["z_a"], z, places=3)

    def test_the_road_passes_under_at_the_plate_level(self):
        """The viaduct's deck is a kind surface: the ground under its middle is the interchange's
        plate, `level` 8 (2 m down), not the deck: 8 m of headroom."""
        h = field(self.build, "height")
        self.assertAlmostEqual(h(100, 100), 8, places=4)

    def test_a_ramp_off_the_deck_level_fails_as_a_gap(self):
        """The same loop with its start written at 1 m below the deck's level (`z0 = 15`): the core
        names the meeting with the deck (`surface.gap`), so a ramp cannot leave a deck unmet."""
        world = os.path.join(self.tmp, "off.toml")
        with open(world, "w") as f:
            f.write(WORLD.replace("{deck_z}", "16").replace('z0        = "= fly.deck_z"', "z0        = 15"))
        build = os.path.join(self.tmp, "off")
        subprocess.run([VIA, "build", world, "--out", build, "--store", os.path.join(self.tmp, "store")],
                       env=self.env, capture_output=True, text=True)
        self.assertIn("surface.gap", self.codes(build))


if __name__ == "__main__":
    unittest.main()
