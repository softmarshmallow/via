"""Small-world cases for the crossing's heads, its deck and its water (lib/ways kinds.toml, `crossing`;
parapet.py; check.py), built by `via`. Skipped unless VIA_BIN names a `via` binary; the SDK's interpreter is VIA_PYTHON (default:
this one).

    VIA_BIN=target/release/via PYTHONPATH=sdk/python python3 -B -m unittest lib/ways/test_crossing_world.py

The world: a beck runs south along x = 60 in a valley whose sides rise 0.08 a metre; a crossing
stands square across it at y = 40, its deck `deck_z` 0.6 m above the ground at its ends, and two
lanes arrive at its ends by name. The deck is the crossing's kind surface (M5 slice 2), so the
beck runs beneath it. The world restates the parapet step and the crossing lint. Every store, config and build goes under one temporary folder,
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
pack    = "{pack}"

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
freeboard = {freeboard}

[feature.lane_west]
kind    = "lane"
path    = [[24, 40], [40, 40]]
ends_at = "bridge.start"

{east}

[step.parapet]
when     = "generate"
worker   = true
for_each = "kind:crossing"
command  = ["python", "-m", "viasdk", "lib:ways/parapet.py"]
params   = {{ asset = "fence_post", spacing = 2 }}

[step.crossings]
when    = "lint"
worker  = true
command = ["python", "-m", "viasdk", "lib:ways/check.py"]
"""

EAST = """[feature.lane_east]
kind      = "lane"
starts_at = "bridge.end"
path      = [[80, 40], [96, 40]]"""
PACK = os.path.join(LIB, "packs", "primitive.toml")

GROUND_AT_ENDS = 20 + 0.08 * 9 + 0.01 * 40          # 21.12 m
DECK_Z = round(GROUND_AT_ENDS + 0.6, 2)             # the deck stands 0.6 m above the lanes' ground


def field(build, name):
    with open(os.path.join(build, "fields", f"{name}.npy"), "rb") as f:
        raw = f.read()
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
    def build_world(cls, name, freeboard=1.2, east=EAST):
        """Builds one variant; returns its build folder."""
        world = os.path.join(cls.tmp, f"{name}.toml")
        with open(world, "w") as f:
            f.write(WORLD.format(deck_z=DECK_Z, freeboard=freeboard, east=east, pack=PACK))
        subprocess.run([VIA, "trust", world, "--yes"], env=cls.env, check=True, capture_output=True)
        build = os.path.join(cls.tmp, name)
        run = subprocess.run([VIA, "build", world, "--out", build, "--store", os.path.join(cls.tmp, "store")],
                             env=cls.env, capture_output=True, text=True)
        assert run.returncode == 0, run.stdout + run.stderr
        return build

    @staticmethod
    def codes(build):
        with open(os.path.join(build, "diagnostics.json")) as f:
            return [(d["code"], d.get("subject")) for d in json.load(f)["diagnostics"]]

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="via-crossing-")
        cls.env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(cls.tmp, "cfg"), VIA_LIB=LIB, TMPDIR=cls.tmp)
        subprocess.run([VIA, "config", "program", "python", os.environ.get("VIA_PYTHON", sys.executable)],
                       env=cls.env, check=True, capture_output=True)
        cls.build = cls.build_world("build")
        cls.h, cls.w = staticmethod(field(cls.build, "height")), staticmethod(field(cls.build, "water"))
        with open(os.path.join(cls.build, "features.json")) as f:
            fs = json.load(f)["features"]
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

    def test_water_runs_beneath_the_deck(self):
        """The deck is a kind surface, not the terrain: under its middle the beck stands at its own
        depth (within 0.05 m of its depth 15 m off), not dry under a causeway as the stand-in made
        it, and the ground there is the beck's bed, not `deck_z`."""
        depth = self.w(60, 40) - self.h(60, 40)
        far = self.w(60, 25) - self.h(60, 25)
        self.assertGreater(depth, 0.1)
        self.assertLessEqual(abs(depth - far), 0.05, (depth, far))
        self.assertLess(self.h(60, 40), DECK_Z - 1)

    def test_each_way_meets_the_deck_level(self):
        """`lane_west.end` meets `bridge.start` and `lane_east.start` meets `bridge.end` (the ends
        lie on the deck, surfaces.md 3.3), each at a gap of 0 with the deck side at `deck_z`."""
        with open(os.path.join(self.build, "meets.json")) as f:
            rows = {(m["a"], m["b"]): m for m in json.load(f)["meets"]}
        for pair, deck_side in ((("lane_west.end", "bridge.start"), "z_b"), (("lane_east.start", "bridge.end"), "z_b")):
            self.assertIn(pair, rows)
            self.assertEqual(rows[pair]["gap"], 0, rows[pair])
            self.assertAlmostEqual(rows[pair][deck_side], DECK_Z, places=6)

    def test_the_lint_passes_and_the_parapet_is_laid(self):
        """check.py judges one crossing and two ends and says nothing else; parapet.py returns its
        two runs (`laid`)."""
        codes = self.codes(self.build)
        self.assertIn(("step.crossings.checked", "step.crossings"), codes)
        self.assertIn(("step.parapet.laid", "feature.bridge"), codes)
        self.assertEqual([c for c in codes if c[0].split(".")[-1] in ("dry", "low_deck", "deck_unmet", "deck_gap")], [])

    def test_the_parapet_is_two_rows_inside_the_deck(self):
        """parapet.py's two runs place 10 posts each (18 m at a 2 m spacing, `ends = "both"`), at
        1.925 m either side of the line: inside the deck's bound (2 m), the post's 0.15 m outer
        face on its edge."""
        with open(os.path.join(self.build, "instances", "names.txt")) as f:
            names = f.read().split()
        ys = self.npy("y")
        rows = sorted({int(ys[i]) for i, n in enumerate(names) if "parapet" in n})
        self.assertEqual(len([n for n in names if "parapet" in n]), 20)
        self.assertEqual(rows, [38075, 41925])

    def test_the_parapet_stands_on_the_deck(self):
        """A run `on` the crossing stands on its one surface, the deck (surfaces.md 4.1): each post
        at its offset, `deck_z - 0.02 * 1.925`, not the beck's bank under the deck."""
        with open(os.path.join(self.build, "instances", "names.txt")) as f:
            names = f.read().split()
        zs = self.npy("z")
        for i, n in enumerate(names):
            if "parapet" in n:
                self.assertAlmostEqual(float(zs[i]), DECK_Z - 0.02 * 1.925, places=3)

    def npy(self, name):
        """One column of the instances (`build.md` 8): a little-endian 1-d array."""
        with open(os.path.join(self.build, "instances", f"{name}.npy"), "rb") as f:
            raw = f.read()
        hlen = struct.unpack("<H", raw[8:10])[0]
        head = raw[10:10 + hlen].decode()
        code = head.split("'descr': '")[1].split("'")[0]
        fmt = {"<i4": "i", "<f4": "f", "<f8": "d"}[code]
        body = raw[10 + hlen:]
        return struct.unpack(f"<{len(body) // struct.calcsize(fmt)}{fmt}", body)

    def test_a_low_deck_and_an_end_met_by_nothing(self):
        """A freeboard of 5 m is more than the deck stands over the beck: `low_deck`. With no lane
        at the east end: `deck_unmet` on `bridge` (the core says nothing of a free path end)."""
        codes = self.codes(self.build_world("low", freeboard=5, east=""))
        self.assertIn(("step.crossings.low_deck", "feature.bridge"), codes)
        self.assertIn(("step.crossings.deck_unmet", "feature.bridge"), codes)
        self.assertEqual(len([c for c in codes if c[0] == "step.crossings.deck_unmet"]), 1)


if __name__ == "__main__":
    unittest.main()
