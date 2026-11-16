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
        # At the default 4 px per metre the inn's text (7 characters, 12.75 m) fits nowhere round it in 16 m.
        self.assertEqual(poi, [{"at": [12.0, 9.0], "box": None, "class": "poi", "floor": None,
                                "icon": "inn", "key": "inn", "kind": "poi", "text": "The Inn"}])
        room = [lb for lb in labels if lb["class"] == "room"]
        self.assertEqual((room[0]["key"], room[0]["floor"], room[0]["at"]), ("hall", 0, [12.0, 9.0]))

    def test_a_room_reads_its_floor_and_number_never_its_id(self):
        wd = world()
        sq2 = [[[10.0, 9.0], [14.0, 9.0], [14.0, 11.0], [10.0, 11.0]]]
        wd["features"].append({"name": "lay#h_s0_r1", "kind": "room", "shape": "region",
                               "attrs": {"storey": "s0"}, "pieces": [sq2]})
        wd["features"].append({"name": "lay#h_s0_r2", "kind": "room", "shape": "region",
                               "attrs": {"storey": "s0", "label": "Kitchen"}, "pieces": [sq2]})
        sq3 = [[[10.0, 7.0], [14.0, 7.0], [14.0, 8.0], [10.0, 8.0]]]
        wd["features"].append({"name": "lay#h_s0_r3", "kind": "room", "shape": "region",
                               "attrs": {"storey": "s0"}, "pieces": [sq3]})
        files, _ = E["make_map"](wd, P)
        rooms = {lb["key"]: lb["text"] for lb in json.loads(files["labels.json"])["labels"]
                 if lb["class"] == "room"}
        # hall is named by hand; of the produced rooms with no text, r1 (y 10) is read before r3 (y 7.5).
        self.assertEqual(rooms, {"lay#h_s0_r1": "Room 0.1", "hall": "Hall", "lay#h_s0_r2": "Kitchen",
                                 "lay#h_s0_r3": "Room 0.2"})

    def test_placed_boxes_never_meet(self):
        labels = [{"key": f"k{i}", "class": "label", "text": "Mill", "at": [50.0 + (i % 3), 50.0]}
                  for i in range(12)]
        E["place_labels"](labels, 1.0, [6, 11], [0, 0, 100, 100])
        boxes = [lb["box"] for lb in labels if lb["box"]]
        for i, a in enumerate(boxes):
            for b in boxes[i + 1:]:
                self.assertFalse(a[0] < b[2] and b[0] < a[2] and a[1] < b[3] and b[1] < a[3], (a, b))
        self.assertEqual(len(boxes), 4)               # the four corners; a side meets two of them
        self.assertEqual(labels[0]["box"], [52.0, 52.0, 78.0, 65.0])  # upper right first

    def test_text_from_a_name(self):
        nt = E["name_text"]
        self.assertEqual((nt("coast.sea_cave"), nt("creek"), nt("city.wards#a0082b5bb57ca744a_0")),
                         ("Sea cave", "Creek", None))

    def test_a_box_covers_no_other_anchor(self):
        lbs = [{"key": "a", "class": "label", "text": "Mill", "at": [50.0, 50.0]},
               {"key": "b", "class": "label", "text": "Ford", "at": [60.0, 58.0]}]   # in a's upper right
        E["place_labels"](lbs, 1.0, [6, 11], [0, 0, 100, 100])
        self.assertEqual(lbs[0]["box"], [22.0, 52.0, 48.0, 65.0])                  # upper left instead

    def test_a_region_label_is_centred_first(self):
        lb = [{"key": "a", "class": "region", "text": "Vale", "at": [50.0, 50.0]}]
        E["place_labels"](lb, 1.0, [6, 11], [0, 0, 100, 100])
        self.assertEqual(lb[0]["box"], [37.0, 43.5, 63.0, 56.5])

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


class Strokes(unittest.TestCase):
    def test_a_way_narrower_than_a_cell_is_never_broken(self):
        wd = world()
        wd["features"][0] = {"name": "lane", "kind": "lane", "shape": "path", "attrs": {"width": 0.2},
                             "points": [[4.3, 1.1], [15.7, 3.9]]}       # a diagonal, 0.2 m wide
        files, _ = E["make_map"](wd, E["params_of"]({"ways": {"lane": "lane"}, "cell": 1.0}))
        idx, rows = load(files)
        lane = next(c["id"] for c in idx["classes"] if c["name"] == "way:lane")
        for k in range(101):
            x, y = 4.3 + k / 100 * 11.4, 1.1 + k / 100 * 2.8
            self.assertEqual(rows[int(y)][int(x)], lane, (x, y))

    def test_a_building_instance_is_a_footprint(self):
        wd = world()
        wd["pack"]["house"] = {"bounds": [-2, -3, 0, 2, 3, 5], "taxon": "building/house"}
        wd["instances"].append({"name": "h1", "asset": "house", "x": 6.0, "y": 9.0, "z": 1.0,
                                "face": 90000, "scale": 1.0})                  # turned: 6 m east-west
        files, _ = E["make_map"](wd, E["params_of"]({"buildings": ["building"], "cell": 1.0}))
        idx, rows = load(files)
        foot = next(c["id"] for c in idx["classes"] if c["name"] == "footprint")
        # x 3 to 9, y 7 to 11: cells by their centres.
        self.assertEqual((rows[7][3], rows[8][8], rows[10][5]), (foot, foot, foot))
        self.assertNotEqual(rows[8][9], foot)
        self.assertNotEqual(rows[11][5], foot)
        fp = json.loads(files["vectors.json"])["footprints"]
        self.assertEqual([(f["name"], f["from"]) for f in fp], [("h1", "instance")])


class Params(unittest.TestCase):
    def test_refused(self):
        for bad in ({"cell": 0}, {"reveal": 3, "cell": 2}, {"tile": 0}, {"ways": ["lane"]}, {"colour": 1},
                    {"glyph": [6]}, {"label_zoom": 0}):
            with self.assertRaises(E["ParamError"]):
                E["params_of"](bad)


if __name__ == "__main__":
    unittest.main()
