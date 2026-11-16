"""Tests for export.py: the transform, ground classes, floors and their links, wall pieces cut at
the plan's height, tile fingerprints, the array file, params.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/map -p 'test_*.py'

The world is handed as read_world returns it (metres), with fields as small grids standing in for
the SDK's arrays (indexed [j, i], or [plane, j, i] for weights).
"""

import json
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
E = runpy.run_path(os.path.join(HERE, "export.py"))


class Grid:
    """A field as the SDK hands it: indexed [j, i] (or [plane, j, i]), with window and materials."""

    def __init__(self, fn, nx, ny, planes=None):
        self.fn, self.window, self.materials = fn, (0, 0, nx, ny), planes

    def __getitem__(self, k):
        return self.fn(*k)


def world(way_end=(16.0, 5.0), wall_z=0.3):
    """A 16 by 12 m world at 0.5 m posts: water west of x = 4, grass east; a lane along y = 5; a
    house (storey s0 at 0.3 m, s1 at 3.3 m) with a stair from 0.3 to 3.3; one wall piece."""
    nx, ny = 33, 25
    h = Grid(lambda j, i: 1.0, nx, ny)
    w = Grid(lambda j, i: 1.5 if i * 0.5 < 4 else 1.0, nx, ny)
    mat = Grid(lambda p, j, i: (255 if p == 0 else 0), nx, ny, planes=["grass", "sand"])
    sq = [[[10.0, 7.0], [14.0, 7.0], [14.0, 11.0], [10.0, 11.0]]]
    feats = [
        {"name": "lane", "kind": "lane", "shape": "path", "attrs": {"width": 2.0},
         "points": [[5.0, 5.0], list(way_end)]},
        {"name": "s0", "kind": "storey", "shape": "region", "attrs": {"level": 0.3}, "pieces": [sq]},
        {"name": "s1", "kind": "storey", "shape": "region", "attrs": {"level": 3.3}, "pieces": [sq]},
        {"name": "hall", "kind": "room", "shape": "region", "attrs": {"storey": "s0"}, "pieces": [sq]},
        {"name": "up", "kind": "flight", "shape": "path", "attrs": {"z0": 0.3, "z1": 3.3},
         "points": [[11.0, 8.0], [13.0, 8.0]]},
        {"name": "door", "kind": "doorway", "shape": "path", "attrs": {"sill_z": 0.3},
         "points": [[12.0, 5.5], [12.0, 7.0]]},
        {"name": "inn", "kind": "poi", "shape": "point", "attrs": {"label": "The Inn", "icon": "inn"},
         "points": [[12.0, 9.0]]},
    ]
    inst = [{"name": "w1", "asset": "wall", "x": 10.0, "y": 9.0, "z": wall_z, "face": 90000, "scale": 1.0},
            {"name": "t1", "asset": "table", "x": 12.0, "y": 9.0, "z": 0.3, "face": 0, "scale": 1.0}]
    pack = {"wall": {"bounds": [-0.1, -0.5, 0, 0.1, 0.5, 2.8], "taxon": "structure/wall"},
            "table": {"bounds": [-0.5, -0.5, 0, 0.5, 0.5, 0.8], "taxon": "furniture"}}
    return {"features": feats, "instances": inst, "pack": pack,
            "fields": {"ground_field": h, "water_field": w, "material": mat},
            "grid": {"extent": [0.0, 0.0, 16.0, 12.0], "resolution": 0.5, "nx": nx, "ny": ny}}


P = E["params_of"]({"ways": {"lane": "lane"}, "floors": {"storey": "level"}, "rooms": {"room": "storey"},
                    "doors": {"doorway": "sill_z"}, "links": {"flight": ["z0", "z1"]},
                    "walls": ["structure"], "tile": 4})


def load(files):
    idx = json.loads(files["map.json"])
    head = files["ground.npy"]
    hl = int.from_bytes(head[8:10], "little")
    nx, ny = idx["transform"]["size"]
    body = head[10 + hl:]
    rows = [list(body[r * nx:(r + 1) * nx]) for r in range(ny)]
    return idx, rows


class Transform(unittest.TestCase):
    def test_cells_and_affine(self):
        t = E["transform"]([0, 0, 10, 6], 2.0)
        self.assertEqual(t["size"], [5, 3])
        self.assertEqual(t["affine"], [[0.5, 0, 0.0], [0, 0.5, 0.0]])
        t = E["transform"]([100, 50, 110, 56], 1.0)
        self.assertEqual(t["affine"], [[1.0, 0, -100.0], [0, 1.0, -50.0]])


class Ground(unittest.TestCase):
    def test_classes_in_order(self):
        files, counts = E["make_map"](world(), P)
        idx, rows = load(files)
        names = [c["name"] for c in idx["classes"]]
        self.assertEqual(names, ["none", "ground:grass", "ground:sand", "water", "way:lane", "footprint"])
        cid = {n: i for i, n in enumerate(names)}
        self.assertEqual(rows[2][1], cid["water"])        # x 1.5, y 2.5: west of 4 m
        self.assertEqual(rows[2][8], cid["ground:grass"])
        self.assertEqual(rows[5][7], cid["way:lane"])     # x 7.5, y 5.5: within 1 m of y = 5
        self.assertEqual(rows[8][11], cid["footprint"])   # inside the house
        self.assertEqual(rows[3][7], cid["ground:grass"])  # x 7.5, y 3.5: 1.5 m off the lane

    def test_array_file_header(self):
        files, _ = E["make_map"](world(), P)
        npy = files["ground.npy"]
        self.assertEqual(npy[:8], b"\x93NUMPY\x01\x00")
        hl = int.from_bytes(npy[8:10], "little")
        self.assertEqual((10 + hl) % 64, 0)
        self.assertIn(b"'shape': (12, 16)", npy[10:10 + hl])
        self.assertEqual(len(npy) - 10 - hl, 12 * 16)


class Floors(unittest.TestCase):
    def test_levels_group_into_floors(self):
        feats = [{"name": f"s{k}", "kind": "storey", "shape": "region", "attrs": {"level": z},
                  "pieces": []} for k, z in enumerate([0.3, 0.309, 0.45, 3.509, 3.55, 6.7, 9.85])]
        fl = E["floors_of"](feats, E["params_of"]({"floors": {"storey": "level"}}))
        self.assertEqual([f["levels"] for f in fl], [[0.3, 0.309, 0.45], [3.509, 3.55], [6.7], [9.85]])
        self.assertEqual([f["z"] for f in fl], [[-0.2, 3.009], [3.009, 6.2], [6.2, 9.35], [9.35, None]])
        self.assertEqual(E["floor_at"](fl, 3.0), 0)
        self.assertEqual(E["floor_at"](fl, 3.509), 1)

    def test_plans_rooms_doors_links_walls(self):
        files, counts = E["make_map"](world(), P)
        f0 = json.loads(files["floors/0.json"])
        f1 = json.loads(files["floors/1.json"])
        self.assertEqual([r["name"] for r in f0["rooms"]], ["hall"])
        self.assertEqual(f1["rooms"], [])
        self.assertEqual([d["name"] for d in f0["doors"]], ["door"])
        self.assertEqual([(ln["name"], ln["from"], ln["to"]) for ln in f0["links"]], [("up", 0, 1)])
        self.assertEqual([(ln["name"], ln["from"], ln["to"]) for ln in f1["links"]], [("up", 0, 1)])
        self.assertEqual([w["instance"] for w in f0["walls"]], ["w1"])   # the table is not a wall
        self.assertEqual(f1["walls"], [])                                 # 2.8 m tall: below 3.3 + 1.2
        # Face 90: the piece's +y (its 1 m length) turns to east.
        xs = sorted(p[0] for p in f0["walls"][0]["outline"])
        self.assertEqual((xs[0], xs[-1]), (9.5, 10.5))
        idx = json.loads(files["map.json"])
        self.assertEqual([f["file"] for f in idx["floors"]], ["floors/0.json", "floors/1.json"])

    def test_wall_outside_the_cut_is_left_out(self):
        files, _ = E["make_map"](world(wall_z=1.6), P)    # its foot over the cut plane at 1.5 m
        self.assertEqual(json.loads(files["floors/0.json"])["walls"], [])


class Labels(unittest.TestCase):
    def test_poi_and_room_labels(self):
        files, _ = E["make_map"](world(), P)
        labels = json.loads(files["labels.json"])["labels"]
        poi = [lb for lb in labels if lb["class"] == "poi"]
        self.assertEqual(poi, [{"at": [12.0, 9.0], "class": "poi", "floor": None, "icon": "inn",
                                "key": "inn", "kind": "poi", "text": "The Inn"}])
        room = [lb for lb in labels if lb["class"] == "room"]
        self.assertEqual((room[0]["key"], room[0]["floor"], room[0]["at"]), ("hall", 0, [12.0, 9.0]))

    def test_label_point_of_a_u_lies_inside(self):
        u = [[[0, 0], [6, 0], [6, 6], [4, 6], [4, 2], [2, 2], [2, 6], [0, 6]]]
        x, y = E["label_point"]([u])
        self.assertTrue(E["in_pieces"]([u], x, y))


class Tiles(unittest.TestCase):
    def test_a_moved_end_touches_only_its_tiles(self):
        a, _ = E["make_map"](world(), P)
        b, _ = E["make_map"](world(way_end=(15.0, 5.0)), P)    # the lane ends 1 m shorter
        ta = {(t["plan"], tuple(t["at"])): t["fingerprint"] for t in json.loads(a["map.json"])["tiles"]["list"]}
        tb = {(t["plan"], tuple(t["at"])): t["fingerprint"] for t in json.loads(b["map.json"])["tiles"]["list"]}
        changed = sorted(k for k in ta if ta[k] != tb[k])
        # Tiles are 4 cells, 4 m: the lane (y 4 to 6, x 4 to 16) lies in tile rows 1; it touched
        # columns 1 to 3, and the cells it no longer covers are in column 3.
        self.assertTrue(changed)
        self.assertTrue(all(k[0] == "ground" and k[1][1] == 1 for k in changed), changed)
        self.assertIn(("ground", (3, 1)), changed)
        self.assertNotIn(("ground", (0, 1)), changed)
        self.assertNotIn(("ground", (2, 1)), changed)                 # the lane is drawn the same there

    def test_same_world_same_bytes(self):
        a, _ = E["make_map"](world(), P)
        b, _ = E["make_map"](world(), P)
        self.assertEqual(a, b)


class Params(unittest.TestCase):
    def test_refused(self):
        for bad in ({"cell": 0}, {"reveal": 3, "cell": 2}, {"tile": 0}, {"ways": ["lane"]}, {"colour": 1}):
            with self.assertRaises(E["ParamError"]):
                E["params_of"](bad)


if __name__ == "__main__":
    unittest.main()
