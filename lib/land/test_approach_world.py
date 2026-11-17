"""Small-world case for the approach (lib/land kinds.toml, `approach`; M7, TODO "a ramp the master owns
cannot meet an anchored place"), built by `via`. Skipped unless VIA_BIN names a `via` binary.

    VIA_BIN=target/release/via python3 -B -m unittest lib/land/test_approach_world.py

The worlds: a place 40 m square, its ground at 4 m and held (`authority = "anchored"` on its import,
composition.md 6.4), stands on a plain at 10 m. A layer ranked above the import writes a point on the
place's south edge, once a `pad` (shaped) and once an `approach` (anchored), each grade 0.1 and reach
30 (the pad's reach is its fixed 10). The pad reads the plain's 10 m (it is ordered before the place's anchored ground) and leaves the
6 m step; the approach reads the place's 4 m and ramps the plain down to it at 1:10, agreeing with the
place's held ground (no `authority.conflict`). Every store, config and build goes under one temporary
folder, removed at the end.
"""

import json
import os
import shutil
import struct
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.dirname(HERE)
VIA = os.environ.get("VIA_BIN")

PLACE = """name    = "place"
extent  = [0, 0, 40, 40]
resolution = 1

[field.height]
role = "surface"
base = "= 4"
"""

PARENT = """name    = "approach_case"
extent  = [0, 0, 120, 120]
resolution = 1
include = ["land"]

[layer.ways]
file  = "ways.toml"
owner = "the way in"
rank  = 1

[field.height]
role = "surface"
base = "= 10"

[import.p]
file      = "place/world.toml"
at        = [40, 60]
authority = "anchored"
"""

WAYS = """[feature.gate_in]                           # on the place's south edge
kind    = "{kind}"
at      = [60, 60]
grade   = 0.1
max_cut = 7
{reach}"""


def field(build, name):
    with open(os.path.join(build, "fields", f"{name}.npy"), "rb") as f:
        raw = f.read()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    rows, cols = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:2])
    body = raw[10 + hlen:]
    vals = struct.unpack(f"<{len(body) // 4}f", body)
    return lambda x, y: vals[round(y) * cols + round(x)]


@unittest.skipUnless(VIA, "VIA_BIN names no via binary")
class ApproachWorld(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="via-approach-")
        cls.env = dict(os.environ, VIA_CONFIG_DIR=os.path.join(cls.tmp, "cfg"), VIA_LIB=LIB, TMPDIR=cls.tmp)
        cls.builds = {k: cls.build_world(k) for k in ("pad", "approach")}

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    @classmethod
    def build_world(cls, kind):
        root = os.path.join(cls.tmp, kind)
        os.makedirs(os.path.join(root, "place"))
        for name, text in (("world.toml", PARENT), ("ways.toml", WAYS.format(kind=kind, reach="reach   = 30\n" if kind == "approach" else "")),
                           (os.path.join("place", "world.toml"), PLACE)):
            with open(os.path.join(root, name), "w") as f:
                f.write(text)
        out = os.path.join(root, "build")
        run = subprocess.run([VIA, "build", os.path.join(root, "world.toml"), "--out", out,
                              "--store", os.path.join(cls.tmp, "store")],
                             env=cls.env, capture_output=True, text=True)
        with open(os.path.join(out, "diagnostics.json")) as f:
            diags = json.load(f)
        recs = diags["diagnostics"] if isinstance(diags, dict) else diags
        return {"exit": run.returncode, "log": run.stderr, "h": field(out, "height"),
                "codes": sorted({r["code"] for r in recs})}

    def test_the_approach_builds_with_no_error_and_no_clash(self):
        b = self.builds["approach"]
        self.assertEqual(b["exit"], 0, b["log"])
        self.assertNotIn("authority.conflict", b["codes"])
        self.assertNotIn("authority.max_change", b["codes"])

    def test_the_approach_ramps_the_plain_to_the_held_place(self):
        h = self.builds["approach"]["h"]
        self.assertAlmostEqual(h(60, 62), 4.0, delta=0.01)         # the place, held
        self.assertAlmostEqual(h(60, 50), 5.0, delta=0.05)         # 10 m out at 1:10
        self.assertAlmostEqual(h(60, 40), 6.0, delta=0.05)         # 20 m out
        self.assertAlmostEqual(h(60, 1), 10.0, delta=0.01)         # past reach and fade: the plain

    def test_a_shaped_pad_reads_the_plain_and_leaves_the_step(self):
        h = self.builds["pad"]["h"]
        self.assertAlmostEqual(h(60, 62), 4.0, delta=0.01)
        self.assertAlmostEqual(h(60, 58), 10.0, delta=0.01)        # the plain 2 m out: a 6 m step


if __name__ == "__main__":
    unittest.main()
