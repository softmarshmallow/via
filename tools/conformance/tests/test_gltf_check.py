"""gltf_check passes a well-formed GLB and fails each broken one (the check able to fail)."""

import json
import os
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from via_conformance import gltf_check  # noqa: E402


def glb(doc, bin_):
    text = json.dumps(doc).encode()
    while len(text) % 4:
        text += b" "
    total = 12 + 8 + len(text) + 8 + len(bin_)
    return (struct.pack("<4sII", b"glTF", 2, total) + struct.pack("<II", len(text), 0x4E4F534A) + text
            + struct.pack("<II", len(bin_), 0x004E4942) + bin_)


def triangle():
    pos = struct.pack("<9f", 0, 0, 0, 1, 0, 0, 0, 0, -1)
    idx = struct.pack("<3H", 0, 1, 2) + b"\0\0"
    doc = {
        "asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "n", "mesh": 0, "rotation": [0, 0, 0, 1],
                   "extras": {"via": {"instance": "n"}}}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, -1], "max": [1, 0, 0]},
            {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"},
        ],
        "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}, {"buffer": 0, "byteOffset": 36, "byteLength": 6}],
        "buffers": [{"byteLength": 44}],
    }
    return doc, pos + idx


class GltfCheck(unittest.TestCase):
    def run_check(self, doc, bin_, *extra):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "x.glb")
            with open(path, "wb") as f:
                f.write(glb(doc, bin_))
            return gltf_check.main([path, *extra])

    def test_good(self):
        self.assertEqual(self.run_check(*triangle()), 0)

    def test_broken(self):
        breaks = {
            "min": lambda d: d["accessors"][0].update(min=[0, 0, 0]),
            "index_ref": lambda d: d["meshes"][0]["primitives"][0].update(indices=5),
            "view_past_buffer": lambda d: d["bufferViews"][1].update(byteLength=60),
            "rotation": lambda d: d["nodes"][0].update(rotation=[0, 1, 0, 1]),
            "version": lambda d: d["asset"].update(version="1.0"),
            "two_parents": lambda d: d["nodes"].extend([{"children": [0]}, {"children": [0]}]),
            "empty_array": lambda d: d.update(materials=[]),
        }
        for name, fn in breaks.items():
            with self.subTest(name):
                doc, bin_ = triangle()
                fn(doc)
                self.assertEqual(self.run_check(doc, bin_), 1)

    def test_index_out_of_range(self):
        doc, bin_ = triangle()
        bin_ = bin_[:36] + struct.pack("<3H", 0, 1, 7) + b"\0\0"
        self.assertEqual(self.run_check(doc, bin_), 1)

    def test_source(self):
        with tempfile.TemporaryDirectory() as d:
            src = os.path.join(d, "s.py")
            with open(src, "w") as f:
                f.write("import numpy\n")
            self.assertTrue(gltf_check.source_faults(src))
            with open(src, "w") as f:
                f.write("import json, struct\n")
            self.assertFalse(gltf_check.source_faults(src))


if __name__ == "__main__":
    unittest.main()
