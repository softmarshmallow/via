"""Small-world cases for the crossing's heads and its water (lib/ways kinds.toml, `crossing`), built
by `via`. Skipped unless VIA_BIN names a `via` binary; the SDK's interpreter is VIA_PYTHON (default:
this one).

    VIA_BIN=target/release/via PYTHONPATH=sdk/python python3 -B -m unittest lib/ways/test_crossing_world.py

The world: a beck runs south along x = 60 in a valley whose sides rise 0.08 a metre; a crossing
stands square across it at y = 40, its deck `deck_z` 0.6 m above the ground at its ends, and two
lanes arrive at its ends by name. Every store, config and build goes under one temporary folder,
removed at the end.
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

WORLD = """name    = "crossing_case"
extent  = [0, 0, 120, 80]
resolution = 1
include = ["water", "ways"]

[field.height]
role = "surface"
base = "= 20 + 0.08 * abs(x - 60) + 0.01 * y"

[field.water]
tags = ["water_level"]
base = "= f('height')"

[feature.beck]
kind = "beck"
path = [[60, 80], [60, 0]]

[feature.bridge_west]
on = {{ ref = "beck", t = 0.5, offset = -9 }}

[feature.bridge_east]
on = {{ ref = "beck", t = 0.5, offset = 9 }}

[feature.bridge]
kind      = "crossing"
starts_at = "bridge_west"
ends_at   = "bridge_east"
path      = []
crosses   = "beck"
width     = 4
deck_z    = {deck_z}
freeboard = 1.2

[feature.lane_west]
kind    = "lane"
path    = [[24, 40], [40, 40]]
ends_at = "bridge.start"

[feature.lane_east]
kind      = "lane"
starts_at = "bridge.end"
path      = [[80, 40], [96, 40]]
"""

GROUND_AT_ENDS = 20 + 0.08 * 9 + 0.01 * 40          # 21.12 m
DECK_Z = round(GROUND_AT_ENDS + 0.6, 2)             # the deck stands 0.6 m above the lanes' ground


def field(build, name):
    raw = open(os.path.join(build, "fields", f"{name}.npy"), "rb").read()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    ny, nx = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    v = struct.unpack(f"<{nx * ny}f", raw[10 + hlen:])

    def at(x, y):                                   # bilinear between the 1 m posts
        i, j = min(int(x), nx - 2), min(int(y), ny - 2)
        a, b = x - i, y - j
        return ((1 - a) * (1 - b) * v[j * nx + i] + a * (1 - b) * v[j * nx + i + 1]
                + (1 - a) * b * v[(j + 1) * nx + i] + a * b * v[(j + 1) * nx + i + 1])
    return at


@unittest.skipUnless(VIA, "VIA_BIN not set: the world cases need a `via` binary")
class CrossingWorld(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="via-crossing-")
        env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(cls.tmp, "cfg"), VIA_LIB=LIB, TMPDIR=cls.tmp)
        subprocess.run([VIA, "config", "program", "python", os.environ.get("VIA_PYTHON", sys.executable)],
                       env=env, check=True, capture_output=True)
        world = os.path.join(cls.tmp, "world.toml")
        with open(world, "w") as f:
            f.write(WORLD.format(deck_z=DECK_Z))
        cls.build = os.path.join(cls.tmp, "build")
        run = subprocess.run([VIA, "build", world, "--out", cls.build, "--store", os.path.join(cls.tmp, "store")],
                             env=env, capture_output=True, text=True)
        assert run.returncode == 0, run.stdout + run.stderr
        cls.h, cls.w = staticmethod(field(cls.build, "height")), staticmethod(field(cls.build, "water"))
        fs = json.load(open(os.path.join(cls.build, "features.json")))["features"]
        cls.bridge = next(f for f in fs if f["name"] == "bridge")

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    def test_heads_bring_the_way_to_the_deck_at_approach(self):
        """`heads`: a way arriving 0.6 m below the deck meets it at `approach` (0.1), not in a 1:2
        lip: the steepest metre of each lane's last 20 m is at most 0.1 (+ 0.01 for the bilinear
        read), and the ground at each end of the deck is `deck_z`."""
        for x0, step in ((51, -1), (69, 1)):
            self.assertAlmostEqual(self.h(x0, 40), DECK_Z, delta=0.01)
            prof = [self.h(x0 + step * s, 40) for s in range(21)]
            steep = max(abs(b - a) for a, b in zip(prof, prof[1:]))
            self.assertLessEqual(steep, 0.11, (x0, [round(p, 2) for p in prof]))

    def test_water_passes_the_deck_at_its_depth(self):
        """`pass`: beside the deck's faces the beck is no deeper than its `flow` (0.3) over the
        ground, as it is 15 m off; before, the stand-in deck pooled it against both faces."""
        half = 2
        for y in (40 - half - 1, 40 + half + 1):
            depth = self.w(60, y) - self.h(60, y)
            far = self.w(60, y + (15 if y > 40 else -15)) - self.h(60, y + (15 if y > 40 else -15))
            self.assertLessEqual(depth, 0.31, y)
            self.assertLessEqual(depth - far, 0.05, (y, depth, far))

    def test_deck_stays_dry(self):
        """`water`: no post of the deck is wet (the level under it is held `freeboard` below)."""
        for x in range(52, 69):
            for y in range(39, 42):
                self.assertLessEqual(self.w(x, y) - self.h(x, y), 0.001, (x, y))


if __name__ == "__main__":
    unittest.main()
