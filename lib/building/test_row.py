"""Tests of lib/building/row.py: the planning functions on plain data, and one build through the
engine (when `VIA_BIN` names a binary).

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_row.py'
"""

import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
R = runpy.run_path(os.path.join(HERE, "row.py"))
L = runpy.run_path(os.path.join(HERE, "lay.py"))


def row(id_, role, size, high=3300, tags=(), shared=None, **params):
    w, d = size
    return {"id": id_, "tags": [f"role:{role}", *tags], "bounds": [-w // 2, -d // 2, 0, w // 2, d // 2, high],
            "params": params, "shared": shared or {}}


CITY = {"module": 3.0, "wall": 0.3, "storey": [3.3, 4.5], "door": 1.4, "parapet": 1.1,
        "stair": {"lane": 1.5, "width": 1.3, "run": 6.0, "landing": 1.5}}


def kit(shared=CITY):
    return [row("bay", "wall", (3000, 300), shared=shared, pitch=3), row("bay_blank", "wall", (3000, 300),
                                                                         tags=("variant:blank",), shared=shared, pitch=3),
            row("half", "filler", (1500, 300), shared=shared, pitch=1.5), row("post", "corner", (300, 300), shared=shared),
            row("door", "door", (1500, 300), shared=shared, pitch=1.5),
            row("tile", "floor", (1500, 1500), 200, shared=shared, pitch=1.5),
            row("parapet", "wall", (3000, 300), 1100, ("storey:parapet",), shared=shared, pitch=3),
            row("parapet_half", "filler", (1500, 300), 1100, ("storey:parapet",), shared=shared, pitch=1.5),
            row("parapet_post", "corner", (300, 300), 1100, ("storey:parapet",), shared=shared)]


FACTS = R["kit_facts"](kit(), "k")
SOUTH = {"to": [78, 16], "kit": "k", "depth": 4, "lots": [{"bays": 4}, {"bays": 3}, {"bays": 5}, {"bays": 3}, {"bays": 4}]}


def plan(rows):
    return R["plan"](rows, lambda taxon: FACTS)


class Facts(unittest.TestCase):
    def test_what_the_kit_states_comes_before_what_its_pieces_measure(self):
        self.assertEqual(FACTS, {"module": 3000, "wall": 300, "reach": 300, "door": 1500, "taxon": "k",
                                 "stair": {"lane": 1500, "width": 1300, "run": 6000, "landing": 1500}})
        loose = R["kit_facts"](kit(shared={}), "k")
        self.assertEqual((loose["module"], loose["wall"], loose["reach"], loose["stair"]), (3000, 300, 300, {}))
        thick = kit(shared={})
        thick[0] = row("bay", "wall", (3200, 400), pitch=3.2)
        self.assertEqual(R["kit_facts"](thick, "k")["module"], 3200)
        self.assertEqual(R["kit_facts"](thick, "k")["wall"], 400)

    def test_a_corner_that_states_its_reach_sizes_the_lots(self):
        rows = kit()
        rows[3] = row("post", "corner", (900, 900), shared=CITY, reach=0.9)
        self.assertEqual(R["kit_facts"](rows, "k")["reach"], 900)

    def test_the_stated_module_and_wall_win_over_the_bays_own(self):
        """A bay re-exported 3.2 m long and 0.4 m thick in a kit that states 3 m and 0.3 m: the
        lots are sized by what the kit states (lay.py rule 11)."""
        rows = kit()
        rows[0] = row("bay", "wall", (3200, 400), shared=CITY, pitch=3.2)
        f = R["kit_facts"](rows, "k")
        self.assertEqual((f["module"], f["wall"], f["reach"]), (3000, 300, 300))

    def test_what_a_kit_states_is_checked(self):
        for shared, word in (({"wall": "thick"}, "shared.wall"), ({"module": -3.0}, "shared.module"),
                             ({"module": 0}, "shared.module"), ({"stair": 2}, "shared.stair"),
                             ({"stair": {"run": "long"}}, "shared.stair.run"), ({"wall": True}, "shared.wall")):
            with self.assertRaises(R["ParamError"]) as c:
                R["kit_facts"](kit(shared=shared), "k")
            self.assertEqual(c.exception.key, "kit")
            self.assertIn(word, c.exception.message)
            self.assertIn("`k`", c.exception.message)
        self.assertEqual(R["kit_facts"](kit(shared={"street": {"kerb": 0.15}}), "k")["module"], 3000)   # another's key

    def test_a_kit_with_no_bay_is_refused(self):
        with self.assertRaises(R["ParamError"]) as c:
            R["kit_facts"]([row("post", "corner", (300, 300))], "k")
        self.assertEqual(c.exception.key, "kit")


class Rows(unittest.TestCase):
    def test_a_row_is_its_lots_wall_to_wall_each_whole_bays_and_two_corners(self):
        """Five lots of 4, 3, 5, 3 and 4 bays on the 3 m module with 0.3 m posts are 12.6, 9.6,
        15.6, 9.6 and 12.6 m: 60 m, the frontage. Each is 12.6 m deep, on the path's left."""
        got, notes = plan([("south", [(18000, 16000)], dict(SOUTH))])
        self.assertEqual(notes, [])
        self.assertEqual([b["key"] for b in got], [f"south_{i}" for i in range(5)])
        self.assertEqual([b["ring"][0][0] for b in got], [18000, 30600, 40200, 55800, 65400])
        self.assertEqual(got[0]["ring"], [(18000, 16000), (30600, 16000), (30600, 28600), (18000, 28600)])
        self.assertEqual(got[-1]["ring"][1], (78000, 16000))

    def test_a_row_is_laid_on_the_left_of_the_way_from_at_to_to(self):
        """East to west along a north street the buildings stand south of it, counter-clockwise
        from the path: side 0 the front, side 2 the back."""
        north = dict(SOUTH, to=[18, 62.8])
        got, _ = plan([("north", [(78000, 62800)], north)])
        self.assertEqual(got[0]["ring"], [(78000, 62800), (65400, 62800), (65400, 50200), (78000, 50200)])
        ring = got[0]["ring"]
        area = sum(ring[i][0] * ring[(i + 1) % 4][1] - ring[(i + 1) % 4][0] * ring[i][1] for i in range(4))
        self.assertGreater(area, 0)

    def test_every_building_closes_on_the_kit(self):
        """What a row is for: no side is worked out by hand, and `lay` closes each one."""
        got, _ = plan([("south", [(18000, 16000)], dict(SOUTH, storeys=3, roof="flat"))])
        rows = kit()
        for b in got:
            out = L["plan"](b["key"], b["ring"], dict(b["attrs"], storey_h=3.3, plinth=0.3, porch=2, walk_out=3), 10.0,
                            None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
            self.assertEqual(out["open"], [], b["key"])
            self.assertFalse([n for n in out["notes"] if n["code"] in ("slab_open", "gap_wide", "roof_none")], b["key"])

    def test_a_row_that_does_not_fill_its_frontage_says_by_how_much(self):
        short = dict(SOUTH, to=[80, 16])
        _, notes = plan([("south", [(18000, 16000)], short)])
        self.assertEqual([(n["code"], n["lots"], n["path"]) for n in notes], [("row_open", 60000, 62000)])
        wide = dict(SOUTH, lots=[{"bays": 4}, {"bays": 4}, {"bays": 5}, {"bays": 3}, {"bays": 4}])
        got, notes = plan([("south", [(18000, 16000)], wide)])
        self.assertEqual([(n["code"], n["lots"] - n["path"]) for n in notes], [("row_open", 3000)])
        self.assertEqual(len(got), 5)                       # nothing is dropped to hide it

    def test_what_a_side_faces(self):
        """Ends between two lots are party walls; the row's two open ends are `end`. A back is a
        party wall where the row round the corner stands along more than half of it."""
        east = {"to": [78, 50.2], "kit": "k", "depth": 4, "lots": [{"bays": 7}]}
        got, _ = plan([("south", [(18000, 16000)], dict(SOUTH)), ("east", [(78000, 28600)], east)])
        faces = {b["key"]: b["attrs"]["faces"] for b in got}
        self.assertEqual(faces["south_0"], ["front", "party", "back", "end"])
        self.assertEqual(faces["south_2"], ["front", "party", "back", "party"])
        self.assertEqual(faces["south_4"], ["front", "end", "party", "party"])    # its back is east_0's end
        self.assertEqual(faces["east_0"], ["front", "end", "back", "party"])

    def test_neighbours_do_not_repeat_and_a_lot_keeps_its_height(self):
        """Rule 5: of `heights`, never the lot before's; the same in every build; and a lot added
        after the others changes none of them."""
        base = dict(SOUTH, heights=[3, 4, 5])
        a = [b["attrs"]["storeys"] for b in plan([("south", [(18000, 16000)], base)])[0]]
        self.assertTrue(all(x in (3, 4, 5) for x in a))
        self.assertTrue(all(x != y for x, y in zip(a, a[1:])), a)
        self.assertEqual(a, [b["attrs"]["storeys"] for b in plan([("south", [(18000, 16000)], dict(base))])[0]])
        more = dict(base, lots=base["lots"] + [{"bays": 3}], to=[87.6, 16])
        self.assertEqual([b["attrs"]["storeys"] for b in plan([("south", [(18000, 16000)], more)])[0]][:5], a)
        two = R["heights_of"]("south", [{"storeys": None}] * 40, [3, 4], 1)
        self.assertTrue(all(x != y for x, y in zip(two, two[1:])))
        self.assertEqual(R["heights_of"]("south", [{"storeys": 6}, {"storeys": None}], [], 2), [6, 2])
        self.assertEqual(R["heights_of"]("south", [{"storeys": 4}, {"storeys": None}], [4], 1), [4, 4])   # one choice only

    def test_the_door_stands_in_its_bay_and_the_stair_along_the_back(self):
        got, _ = plan([("south", [(18000, 16000)], dict(SOUTH, storeys=3))])
        self.assertEqual(got[0]["attrs"]["doors"], [{"storey": 0, "side": 0, "at": 4.05}])
        self.assertEqual(got[0]["attrs"]["stair"], {"side": 2, "at": 1.8})
        first = dict(SOUTH, storeys=3, lots=[{"bays": 4, "door": 0}])
        self.assertEqual(plan([("s", [(18000, 16000)], dict(first, to=[30.6, 16]))])[0][0]["attrs"]["doors"][0]["at"], 1.05)
        one = dict(SOUTH, storeys=1, lots=[{"bays": 4}], to=[30.6, 16])
        self.assertNotIn("stair", plan([("s", [(18000, 16000)], one)])[0][0]["attrs"])

    def test_a_narrow_lot_takes_the_stair_along_its_side_or_is_refused(self):
        """The city kit's stairwell is 9.6 m with its two landings: a lot two bays wide (6.6 m)
        takes it along its side, 12.6 m deep; in a row two bays deep no side holds it."""
        narrow = dict(SOUTH, storeys=3, lots=[{"bays": 2}], to=[24.6, 16])
        got, notes = plan([("s", [(18000, 16000)], narrow)])
        self.assertEqual((got[0]["attrs"]["stair"], notes), ({"side": 1, "at": 1.8}, []))
        got, notes = plan([("s", [(18000, 16000)], dict(narrow, depth=2))])
        self.assertEqual((got, [(n["code"], n["lot"]) for n in notes]), ([], [("row_stair", 0)]))
        two = dict(narrow, depth=2, storeys=2)
        self.assertEqual(plan([("s", [(18000, 16000)], two)])[0], [])

    def test_the_stair_keeps_a_landing_past_its_head_on_two_storeys_too(self):
        """A flight that ends on the far wall leaves `lay` no wall to close there. A side of
        8.1 m holds a landing and the city kit's run between two walls and no more: refused for
        two storeys as for three. 9.6 m holds the second landing."""
        self.assertIs(R["stair_of"]([8100, 8100, 8100, 8100], 2, FACTS), False)
        self.assertIs(R["stair_of"]([8100, 8100, 8100, 8100], 3, FACTS), False)
        self.assertEqual(R["stair_of"]([9600, 12600, 9600, 12600], 2, FACTS), {"side": 2, "at": 1.8})
        self.assertEqual(R["stair_of"]([8100, 9600, 8100, 9600], 2, FACTS), {"side": 1, "at": 1.8})
        self.assertEqual(R["stair_of"]([8100, 8100, 8100, 9600], 4, FACTS), {"side": 3, "at": 1.8})
        self.assertIsNone(R["stair_of"]([3600, 3600, 3600, 3600], 1, FACTS))

    def test_two_rows_that_both_run_to_a_corner(self):
        """The west row written along the whole west side: its end lots stand where the south
        row's corner building does. The lot of the row named later is not returned, and said."""
        west = {"to": [18, 16], "kit": "k", "depth": 4, "lots": [{"bays": 7}, {"bays": 4}]}
        got, notes = plan([("south", [(18000, 16000)], dict(SOUTH)), ("west", [(18000, 50200)], west)])
        self.assertEqual([(n["code"], n["row"], n["lot"], n["others"]) for n in notes],
                         [("row_meets", "west", 1, ["south_0"])])
        self.assertEqual(notes[0]["clear"], {"key": "to", "point": (18000, 28600)})   # end the row where the lot began
        self.assertEqual(sorted(b["key"] for b in got if b["row"] == "west"), ["west_0"])

    def test_what_a_row_passes_on(self):
        fill = [{"faces": ["party"], "pattern": ["blank"]}]
        got, _ = plan([("s", [(18000, 16000)], dict(SOUTH, storey_h=3.3, ground_h=4.5, roof="flat", fill=fill))])
        a = got[0]["attrs"]
        self.assertEqual((a["storey_h"], a["ground_h"], a["roof"], a["fill"], a["kit"]), (3.3, 4.5, "flat", fill, "k"))
        self.assertNotIn("plinth", a)                      # what the row does not write is the building's default
        more = dict(SOUTH, top_h=4.0, plinth=0.6, porch=3, walk_out=5)
        a = plan([("s", [(18000, 16000)], more)])[0][0]["attrs"]
        self.assertEqual((a["top_h"], a["plinth"], a["porch"], a["walk_out"]), (4.0, 0.6, 3, 5))

    def test_a_back_is_a_party_wall_only_where_more_than_half_of_it_is(self):
        """One building 15.6 m long. A row round the corner whose end lies along 3 m of its back
        leaves it a back; one that lies along 12.6 m of it makes it a party wall. The other row's
        own end is a party wall either way: a metre is enough for an end."""
        one = {"to": [15.6, 0], "kit": "k", "depth": 4, "lots": [{"bays": 5}]}
        up = {"kit": "k", "depth": 4, "lots": [{"bays": 4}]}
        for x, word in ((3.0, "back"), (12.6, "party"), (7.5, "back"), (8.1, "party")):
            got, notes = plan([("a", [(0, 0)], dict(one)), ("b", [(int(x * 1000), 12600)], dict(up, to=[x, 25.2]))])
            self.assertEqual(notes, [])
            faces = {b["key"]: b["attrs"]["faces"] for b in got}
            self.assertEqual(faces["a_0"][2], word, x)
            self.assertEqual(faces["b_0"], ["front", "end", "back", "party"], x)

    def test_rows_off_the_axes_find_their_party_walls_and_do_not_meet_by_rounding(self):
        """The block's south and east rows turned about its corner by 10, 30 and 45 degrees: a
        ring rounded to the millimetre is not parallel to its neighbour's, so what stands beside
        what is told from the exact rings. Every party wall is found and no `row_meets` is said
        of rows that only touch."""
        import math
        east = {"kit": "k", "depth": 4, "lots": [{"bays": 7}]}
        for deg in (0, 10, 30, 45, 73):
            c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))

            def turn(x, y):
                return (18 + (x - 18) * c - (y - 16) * s, 16 + (x - 18) * s + (y - 16) * c)
            a, b = turn(78, 28.6), turn(78, 50.2)
            rows = [("south", [(18000, 16000)], dict(SOUTH, to=list(turn(78, 16)))),
                    ("east", [(int(round(a[0] * 1000)), int(round(a[1] * 1000)))], dict(east, to=list(b)))]
            got, notes = plan(rows)
            self.assertEqual([n["code"] for n in notes], [], deg)
            faces = {x["key"]: x["attrs"]["faces"] for x in got}
            self.assertEqual(faces["south_4"], ["front", "end", "party", "party"], deg)
            self.assertEqual(faces["east_0"], ["front", "end", "back", "party"], deg)
            self.assertEqual(faces["south_2"], ["front", "party", "back", "party"], deg)

    def test_a_fill_names_only_the_storeys_a_building_has(self):
        """A row's lots differ in height, and `lay` refuses a storey a building has not: one
        line on the row may fill the fifth storey of those that have one."""
        fill = [{"storeys": [4], "pattern": ["blank"]}, {"storeys": [0, 3], "pattern": ["blank"]},
                {"faces": ["party"], "pattern": ["blank"]}]
        lots = [{"bays": 4, "storeys": 3}, {"bays": 3, "storeys": 5}]
        got, _ = plan([("s", [(18000, 16000)], dict(SOUTH, lots=lots, to=[40.2, 16], fill=fill))])
        self.assertEqual(got[0]["attrs"]["fill"], [{"storeys": [0], "pattern": ["blank"]}, fill[2]])
        self.assertEqual(got[1]["attrs"]["fill"], fill)
        self.assertEqual(R["fill_for"]([{"storeys": [2]}], 2), [])

    def test_a_fault_is_said_of_its_row(self):
        with self.assertRaises(R["ParamError"]) as c:
            plan([("south", [(18000, 16000)], dict(SOUTH)), ("west", [(18000, 50200)], dict(SOUTH, lots=[]))])
        self.assertEqual((c.exception.row, c.exception.key), ("west", "lots"))

    def test_the_hint_of_two_rows_meeting_counts_lots_not_buildings(self):
        """A first lot refused for its stair is still the first lot: the second lot's hint is
        not the one for a row's first. And a first lot that meets another row is told the `at`
        past it."""
        west = {"to": [18, 16], "kit": "k", "depth": 4, "lots": [{"bays": 4}, {"bays": 7}, {"bays": 4}]}
        _, notes = plan([("south", [(18000, 16000)], dict(SOUTH)),
                         ("north", [(78000, 62800)], dict(SOUTH, to=[18, 62.8])),
                         ("west", [(18000, 62800)], west)])
        meets = {n["lot"]: n["clear"] for n in notes if n["code"] == "row_meets"}
        self.assertEqual(meets, {0: {"key": "at", "point": (18000, 50200)}, 2: {"key": "to", "point": (18000, 28600)}})
        shallow = {"to": [18, 16], "kit": "k", "depth": 2, "storeys": 3, "lots": [{"bays": 2}, {"bays": 9}]}
        _, notes = plan([("south", [(18000, 16000)], dict(SOUTH)), ("west", [(18000, 50200)], shallow)])
        self.assertEqual([(n["code"], n["lot"]) for n in notes], [("row_stair", 0), ("row_meets", 1)])
        self.assertEqual(notes[1]["clear"]["key"], "to")

    def test_bad_rows_are_refused_by_name(self):
        for change, key in (({"lots": []}, "lots"), ({"lots": [{"bays": 0}]}, "lots"), ({"lots": [{"bays": 2.5}]}, "lots"),
                            ({"lots": [{"bays": 3, "door": 3}]}, "lots"), ({"lots": [{"bays": 3, "wide": 1}]}, "lots"),
                            ({"depth": 0}, "depth"), ({"heights": [0]}, "heights"), ({"to": [18, 16]}, "to"),
                            ({"to": [1]}, "to"), ({"kit": ""}, "kit"), ({"storeys": 0}, "storeys")):
            with self.assertRaises(R["ParamError"]) as c:
                plan([("south", [(18000, 16000)], dict(SOUTH, **change))])
            self.assertEqual(c.exception.key, key, change)

    def test_whole_numbers_read_through_the_engine_are_floats(self):
        got, _ = plan([("s", [(18000, 16000)], dict(SOUTH, depth=4.0, lots=[{"bays": 4.0, "storeys": 3.0}], to=[30.6, 16]))])
        self.assertEqual((got[0]["attrs"]["storeys"], type(got[0]["attrs"]["storeys"])), (3, int))


class Faces(unittest.TestCase):
    """lay.py rule 12: `fill` by what a side faces, and rule 10's parapet."""

    RING = [(18000, 16000), (30600, 16000), (30600, 28600), (18000, 28600)]

    def lay(self, **attrs):
        rows = kit()
        a = {"storeys": 2, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}], "stair": {"side": 2, "at": 1.8}, **attrs}
        return L["plan"]("b", self.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))

    @staticmethod
    def pieces(out, side, storey=1):
        got = []
        for r in out["runs"]:
            if r["key"].startswith(f"b_s{storey}_w{side}_"):
                got += r.get("assets") or [r["asset"]] * r["count"]
        return got

    def test_fill_by_what_a_side_faces(self):
        out = self.lay(faces=["front", "party", "back", "party"], fill=[{"faces": ["party"], "pattern": ["blank"]}])
        self.assertEqual(set(self.pieces(out, 1)), {"bay_blank"})
        self.assertEqual(set(self.pieces(out, 3)), {"bay_blank"})
        self.assertEqual(set(self.pieces(out, 0)), {"bay"})
        self.assertEqual(set(self.pieces(out, 2)), {"bay"})

    def test_a_word_no_side_has_matches_none(self):
        out = self.lay(faces=["front", "party", "back", "party"], fill=[{"faces": ["end"], "pattern": ["blank"]}])
        self.assertFalse(any("bay_blank" in self.pieces(out, k) for k in range(4)))

    def test_faces_and_fill_are_checked(self):
        for attrs, key, word in (({"faces": ["front", "back"]}, "faces", "each of the outline's 4 sides"),
                                 ({"faces": "front"}, "faces", "each of the outline's"),
                                 ({"fill": [{"faces": ["party"], "pattern": ["blank"]}]}, "fill", "says of no side"),
                                 ({"faces": ["front", "party", "back", "party"],
                                   "fill": [{"faces": ["party"], "sides": [1], "pattern": ["blank"]}]}, "fill", "both"),
                                 ({"faces": ["front", "party", "back", "party"],
                                   "fill": [{"faces": "party", "pattern": ["blank"]}]}, "fill", "is not")):
            with self.assertRaises(L["ParamError"]) as c:
                self.lay(**attrs)
            self.assertEqual(c.exception.key, key, attrs)
            self.assertIn(word, c.exception.message)

    def test_a_flat_roof_gets_the_kits_parapet(self):
        """The parapet is the kit's `storey:parapet` wall pieces round the outline, on the roof,
        closing as the walls under it do: 4 + 4 + 4 + 4 bays and 4 posts here."""
        out = self.lay(roof="flat")
        runs = [r for r in out["runs"] if r["key"].startswith("b_roof_w")]
        self.assertEqual(sorted((r["key"], r["asset"], r["count"], r["ref"]) for r in runs),
                         [(f"b_roof_w{k}_0", "parapet", 4, "b_roof") for k in range(4)])
        posts = [i for i in out["instances"] if i["key"].startswith("b_roof_c")]
        self.assertEqual([(i["asset"], i["surface"]) for i in posts], [("parapet_post", "b_roof.garden")] * 4)
        self.assertEqual(out["open"], [])

    def test_no_parapet_without_the_pieces_or_the_roof(self):
        rows = [r for r in kit() if "storey:parapet" not in r["tags"]]
        a = {"storeys": 1, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k", "roof": "flat",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}]}
        out = L["plan"]("b", self.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertFalse([r for r in out["runs"] if "_roof_w" in r["key"]])
        self.assertFalse([r for r in self.lay()["runs"] if "_roof_w" in r["key"]])           # no roof asked for
        self.assertFalse([r for r in self.lay(roof="garden")["runs"] if "_roof_w" in r["key"]])

    def test_a_gable_that_falls_back_to_a_flat_roof_gets_no_parapet(self):
        """The parapet goes round a flat roof that was asked for. A gable this kit cannot lay
        falls back to the tiles, and is said (`roof_none`): no wall is put round what was meant
        to be a pitched roof."""
        out = self.lay(roof="gable")
        self.assertTrue([n for n in out["notes"] if n["code"] == "roof_none"])
        self.assertFalse([r for r in out["runs"] if "_roof_w" in r["key"]])

    def test_an_open_parapet_is_said_of_the_parapet(self):
        rows = kit()
        a = {"storeys": 1, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k", "roof": "flat",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}]}
        ring = [(18000, 16000), (31000, 16000), (31000, 28600), (18000, 28600)]            # 13 m: no whole bays
        out = L["plan"]("b", ring, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertTrue([o for o in out["open"] if o.get("parapet")])
        self.assertTrue([o for o in out["open"] if not o.get("parapet")])

    def test_a_storeys_walls_never_take_a_parapet_piece(self):
        out = self.lay(roof="flat")
        walls = [a for k in range(4) for s in (0, 1) for a in self.pieces(out, k, s)]
        self.assertTrue(walls)
        self.assertFalse([a for a in walls if a.startswith("parapet")])


class Foot(unittest.TestCase):
    """lay.py rule 13: the wall's foot against the ground, and the kit's base."""

    RING = [(18000, 16000), (30600, 16000), (30600, 28600), (18000, 28600)]

    @staticmethod
    def rows(base=1500):
        rows = [r for r in kit() if "storey:parapet" not in r["tags"]]
        if base:
            rows += [row("base", "wall", (3000, 300), base, ("storey:base",), shared=CITY, pitch=3),
                     row("base_half", "filler", (1500, 300), base, ("storey:base",), shared=CITY, pitch=1.5),
                     row("base_post", "corner", (300, 300), base, ("storey:base",), shared=CITY)]
        return rows

    def lay(self, slope=0.0, base=1500, **attrs):
        """The building on ground that rises `slope` metres a metre to the east, from 10 m at its
        west side; `foot` as `read_foot` gives it."""
        rows = self.rows(base)
        a = {"storeys": 1, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}], **attrs}
        es = L["edges"](self.RING)
        foot = []
        for e in es:
            n = int(round(e["L"] / 500))
            for i in range(n):
                x, y = L["rnd"](L["local"](e, (i + 0.5) * e["L"] / n, 0))
                foot.append((x, y, 10.0 + slope * (x - 18000) / 1000))
        mean = sum(h for _, _, h in foot) / len(foot)
        return L["plan"]("b", self.RING, a, mean, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h), foot=foot)

    def test_the_base_is_laid_round_the_outline_under_the_door_too(self):
        out = self.lay()
        runs = sorted((r["key"], r["asset"], r["count"], r["ref"]) for r in out["runs"] if "_base_w" in r["key"])
        self.assertEqual(runs, [(f"b_base_w{k}_0", "base", 4, "b_s0") for k in range(4)])
        posts = [i for i in out["instances"] if i["key"].startswith("b_base_c")]
        self.assertEqual([(i["asset"], i["surface"]) for i in posts], [("base_post", "b_s0.floor")] * 4)
        self.assertFalse([n for n in out["notes"] if n["code"] in ("foot_open", "base_short")])

    def test_a_base_that_reaches_the_ground_says_nothing_on_a_slope(self):
        """A rise of 0.08 m a metre over 12.6 m: the floor is 0.3 m over the mean, so 0.8 m over
        the ground at the low side. A base 1.5 m deep reaches it."""
        out = self.lay(slope=0.08)
        self.assertFalse([n for n in out["notes"] if n["code"] in ("foot_open", "base_short")])

    def test_a_base_too_shallow_says_how_far_it_stops_short(self):
        out = self.lay(slope=0.08, base=500)
        (note,) = [n for n in out["notes"] if n["code"] == "base_short"]
        self.assertAlmostEqual(note["hang"], 0.3 + 0.08 * 6.3, places=2)      # the plinth and half the rise, at the low side
        self.assertEqual(note["deep"], 0.5)
        self.assertGreater(note["length"], 12.0)
        self.assertEqual(note["at"][0], 18000)                                               # the low, west side

    def test_a_kit_with_no_base_says_the_foot_is_open(self):
        level = self.lay(base=0)
        (note,) = [n for n in level["notes"] if n["code"] == "foot_open"]
        self.assertEqual((note["hang"], note["plinth"]), (0.3, 0.3))                         # the plinth alone: said as an info
        self.assertFalse([r for r in level["runs"] if "_base_w" in r["key"]])
        sloped = self.lay(slope=0.08, base=0)
        (note,) = [n for n in sloped["notes"] if n["code"] == "foot_open"]
        self.assertGreater(note["hang"], 0.7)
        self.assertEqual(self.lay(base=0, plinth=0)["notes"], [n for n in self.lay(base=0, plinth=0)["notes"]
                                                               if n["code"] not in ("foot_open", "base_short")])

    def test_five_centimetres_of_air_is_not_said(self):
        """`FOOT_SEEN`: on the 0.08 slope the floor is 0.804 m over the ground at the low side. A
        base 0.8 m deep is 4 mm short of it, and nothing is said; one 0.7 m deep is."""
        self.assertFalse([n for n in self.lay(slope=0.08, base=800)["notes"] if n["code"] == "base_short"])
        self.assertTrue([n for n in self.lay(slope=0.08, base=700)["notes"] if n["code"] == "base_short"])

    def test_what_a_foot_note_is_said_as(self):
        self.assertEqual(L["foot_word"]({"code": "base_short", "hang": 0.9, "plinth": 0.3}), "base_short")
        self.assertEqual(L["foot_word"]({"code": "foot_open", "hang": 0.3, "plinth": 0.3}), "plinth_bare")
        self.assertEqual(L["foot_word"]({"code": "foot_open", "hang": 0.34, "plinth": 0.3}), "plinth_bare")
        self.assertEqual(L["foot_word"]({"code": "foot_open", "hang": 0.36, "plinth": 0.3}), "foot_open")
        self.assertEqual(L["foot_word"]({"code": "foot_open", "hang": 0.04, "plinth": 0}), "plinth_bare")

    def test_the_ground_is_read_along_every_side(self):
        """`read_foot`: a point every 0.5 m from a quarter metre in, on all four sides."""
        class Field:
            name = "height"

        class Fake:
            def fields(self, roles, window=None):
                return [Field()]

            def eval(self, formula, x, y):
                self.formula = formula
                return [10.0 + 0.001 * a for a in x]
        step = Fake()
        foot = L["read_foot"](step, self.RING)
        self.assertEqual(step.formula, "= f('height')")
        self.assertEqual(len(foot), 2 * (25 + 25))                    # 12.6 m a side: 25 points each
        self.assertEqual(foot[0], (18252, 16000, 10.0 + 18.252))
        self.assertEqual({(x == 18000, x == 30600, y == 16000, y == 28600) for x, y, _ in foot},
                         {(True, False, False, False), (False, True, False, False),
                          (False, False, True, False), (False, False, False, True)})

    def test_no_ground_read_means_nothing_said(self):
        rows = self.rows(0)
        a = {"storeys": 1, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}]}
        out = L["plan"]("b", self.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertFalse([n for n in out["notes"] if n["code"] in ("foot_open", "base_short")])

    def test_a_storeys_walls_never_take_a_base_piece(self):
        out = self.lay()
        walls = [r.get("asset") or r["assets"][0] for r in out["runs"] if "_s0_w" in r["key"]]
        self.assertTrue(walls)
        self.assertFalse([a for a in walls if a.startswith("base")])


class Outlines(unittest.TestCase):
    """lay.py rule 14: a storey with its own outline."""

    RING = [(0, 0), (12600, 0), (12600, 12600), (0, 12600)]

    def lay(self, storeys=3, **attrs):
        rows = kit()
        a = {"storeys": storeys, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k", "roof": "flat",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}], "stair": {"side": 2, "at": 1.8}, **attrs}
        return L["plan"]("b", self.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))

    def test_a_ring_moved_side_by_side(self):
        self.assertEqual(L["shift_ring"](self.RING, [3000, 0, 0, 0]), [(0, 3000), (12600, 3000), (12600, 12600), (0, 12600)])
        self.assertEqual(L["shift_ring"](self.RING, [-1500, 0, 300, 0]), [(0, -1500), (12600, -1500), (12600, 12300), (0, 12300)])
        turned = [(0, 0), (8000, 6000), (2000, 14000), (-6000, 8000)]                     # a 10 m square at 36.87 degrees
        self.assertEqual(L["shift_ring"](turned, [2500, 0, 0, 0])[:2], [(-1500, 2000), (6500, 8000)])

    def test_the_top_storey_set_back_a_bay(self):
        """Storey 2 stands 3 m inside the front: its side walls are three bays where the building's
        are four, its front stands on the line 3 m in, and the roof covers its own outline."""
        out = self.lay(setback=[{"storey": 2, "sides": [0], "by": 3.0}])
        self.assertEqual(out["storeys"][1]["region"][0], (0, 0))                          # storey 1: the footprint
        count = {r["key"]: r["count"] for r in out["runs"]}
        self.assertEqual((count["b_s1_w1_0"], count["b_s2_w1_0"], count["b_s2_w3_0"]), (4, 3, 3))
        self.assertEqual(count["b_s2_w0_0"], 4)
        front = next(r for r in out["runs"] if r["key"] == "b_s2_w0_0")
        self.assertEqual(front["ref"], "b_s2")
        ys = [p[1] for p in out["storeys"][2]["region"]]
        self.assertEqual(min(ys), 3000)
        self.assertEqual(out["roof"]["region"], [(0, 3000), (12600, 3000), (12600, 12600), (0, 12600)])
        self.assertFalse([o for o in out["open"] if not o.get("parapet")])               # every storey's walls close

    def test_the_terrace_is_a_roof_at_the_storeys_level_with_its_parapet_on_three_sides(self):
        out = self.lay(setback=[{"storey": 2, "sides": [0], "by": 3.0}])
        (t,) = out["terraces"]
        self.assertEqual((t["key"], t["region"]), ("b_t2_0", [(0, 0), (12600, 0), (12600, 3000), (0, 3000)]))
        self.assertEqual(t["eaves"], out["storeys"][2]["level"])
        runs = sorted(r["key"] for r in out["runs"] if r["key"].startswith("b_t2_0_w"))
        self.assertEqual(runs, ["b_t2_0_w0_0"])                                          # the front; none against the wall above
        posts = [i for i in out["instances"] if i["key"].startswith("b_t2_0_c")]
        self.assertEqual(len(posts), 4)
        self.assertTrue(all(i["surface"] == "b_t2_0.garden" for i in posts))

    def test_what_a_terrace_of_whole_bays_cannot_close_is_said(self):
        """3 m of set-back is 2.7 m of terrace inside its parapet and 2.4 m of parapet between
        two posts: no whole number of 1.5 m tiles or pieces. Both are said, neither is fitted."""
        out = self.lay(setback=[{"storey": 2, "sides": [0], "by": 3.0}])
        notes = [n for n in out["notes"] if n["code"] == "slab_open" and n.get("terrace")]
        self.assertEqual([(n["storey"], n["deep"]) for n in notes], [(2, 2700)])
        opens = sorted((o["side"], o["length"]) for o in out["open"] if o.get("parapet"))
        self.assertEqual(opens, [(1, 2400), (3, 2400)])
        tiles = [i for i in out["instances"] if i["key"].startswith("b_t2_0_f")]
        self.assertEqual(len(tiles), 8)                                                  # one row of whole tiles, 12 m

    def test_an_overhang_is_the_storeys_own_floor(self):
        out = self.lay(storeys=2, overhang=[{"storey": 1, "sides": [0], "by": 1.5}])
        self.assertEqual(min(p[1] for p in out["storeys"][1]["region"]), -1500)
        self.assertEqual(out["terraces"], [])
        count = {r["key"]: r["count"] for r in out["runs"]}
        self.assertEqual(count["b_s1_w0_0"], 4)
        tiles = [i for i in out["instances"] if i["key"].startswith("b_s1_f") and i["at"][1] < 0]
        self.assertTrue(tiles)                                                           # the floor over the street
        self.assertEqual(out["roof"]["region"][0], (0, -1500))

    def test_an_overhang_the_kit_cannot_close_is_an_open_wall(self):
        """A jetty of 0.45 m makes the side walls 13.05 m: no whole number of half bays."""
        out = self.lay(storeys=2, overhang=[{"storey": 1, "sides": [0], "by": 0.45}])
        opens = sorted((o["side"], o["storey"]) for o in out["open"] if not o.get("parapet"))
        self.assertEqual(opens, [(1, 1), (3, 1)])
        self.assertEqual(sorted(o["side"] for o in out["open"] if o.get("parapet")), [1, 3])   # and the parapet over them

    def test_a_set_back_holds_from_its_storey_up(self):
        out = self.lay(storeys=4, setback=[{"storey": 2, "sides": [0], "by": 3.0}])
        low = [min(p[1] for p in st["region"]) for st in out["storeys"]]
        self.assertEqual(low, [0, 0, 3000, 3000])
        self.assertEqual([t["key"] for t in out["terraces"]], ["b_t2_0"])                 # one terrace, not one a storey

    def test_the_roof_covers_the_top_storeys_outline(self):
        """The flat roof's tiles and parapet, and a gable's slopes and gable halves, stand over
        the set-back storey and not over the terrace before it."""
        out = self.lay(setback=[{"storey": 2, "sides": [0], "by": 3.0}])
        tiles = [i for i in out["instances"] if i["key"].startswith("b_roof_f")]
        self.assertTrue(tiles)
        self.assertGreaterEqual(min(i["at"][1] for i in tiles), 3000 + 300)
        posts = sorted(i["at"] for i in out["instances"] if i["key"].startswith("b_roof_c"))
        self.assertEqual(posts[0], (150, 3150))
        rows = kit() + [row("slope", "roof_slope", (1500, 4800), 3000, pitch=1.5, span=4.8),
                        row("gable", "roof_gable", (4800, 300), 3000, span=4.8)]
        a = {"storeys": 3, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k", "roof": "gable",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}], "stair": {"side": 2, "at": 1.8},
             "setback": [{"storey": 2, "sides": [0], "by": 3.0}]}
        out = L["plan"]("b", self.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertFalse([n for n in out["notes"] if n["code"] == "roof_none"])            # 9.6 m deep: twice the span
        gables = [i["at"] for i in out["instances"] if i["asset"] == "gable"]
        self.assertEqual(len(gables), 4)
        self.assertGreaterEqual(min(y for _, y in gables), 3000)

    def test_a_terrace_is_named_by_its_own_key_whatever_the_building_is_called(self):
        """A building whose name holds `_w2_` kept no parapet on its terrace's front: the runs
        against the wall above were dropped by a search of the whole key."""
        rows = kit()
        a = {"storeys": 3, "storey_h": 3.3, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k", "roof": "flat",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}], "stair": {"side": 2, "at": 1.8},
             "setback": [{"storey": 2, "sides": [0], "by": 3.0}]}
        out = L["plan"]("a_w2_b", self.RING, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertEqual(sorted(r["key"] for r in out["runs"] if r["key"].startswith("a_w2_b_t2_0_w")), ["a_w2_b_t2_0_w0_0"])

    def test_what_is_said_of_a_terrace_names_the_buildings_own_sides(self):
        out = self.lay(setback=[{"storey": 2, "sides": [0], "by": 3.0}])
        opens = sorted((o["side"], o["strip"], o["terrace"]) for o in out["open"] if o.get("terrace") is not None)
        self.assertEqual(opens, [(1, 1, 2), (3, 3, 2)])            # the building's sides 1 and 3, not the strip's

    def test_sides_that_meet_at_a_corner_are_not_moved_together(self):
        for sb in ([{"storey": 1, "sides": [0, 1], "by": 1.5}],
                   [{"storey": 1, "sides": [0], "by": 1.5}, {"storey": 1, "sides": [1], "by": 1.5}],
                   [{"storey": 1, "sides": [3], "by": 1.5}, {"storey": 1, "sides": [0], "by": 1.5}]):
            with self.assertRaises(L["ParamError"]) as c:
                L["offsets_of"]({"setback": sb}, 2, self.RING)
            self.assertIn("two sides that meet at a corner", c.exception.message)
        self.assertEqual(L["offsets_of"]({"setback": [{"storey": 1, "sides": [0, 2], "by": 1.5}]}, 2, self.RING),
                         [[0, 0, 0, 0], [1500, 0, 1500, 0]])
        self.assertEqual(L["offsets_of"]({"setback": [{"storey": 1, "sides": [0], "by": 1.5}],
                                          "overhang": [{"storey": 1, "sides": [2], "by": 0.6}]}, 3, self.RING)[2],
                         [1500, 0, -600, 0])

    def test_what_may_not_be_moved(self):
        for attrs, word in (({"setback": [{"storey": 2, "sides": [2], "by": 3.0}]}, "the stair climbs side 2"),
                            ({"setback": [{"storey": 2, "sides": [1], "by": 3.0}]}, "the stair climbs side 2"),
                            ({"setback": [{"storey": 2, "sides": [3], "by": 3.0}]}, "the stair climbs side 2"),
                            ({"setback": [{"storey": 0, "sides": [0], "by": 3.0}]}, "is not"),
                            ({"setback": [{"storey": 3, "sides": [0], "by": 3.0}]}, "names storey 3"),
                            ({"setback": [{"storey": 1, "sides": [0], "by": -1}]}, "is not"),
                            ({"setback": [{"storey": 1, "side": 0, "by": 3}]}, "is not"),
                            ({"setback": "front"}, "list of tables"),
                            ({"setback": [{"storey": 1, "sides": [0], "by": 12.0}]}, "less than a metre deep")):
            with self.assertRaises(L["ParamError"]) as c:
                self.lay(**attrs)
            self.assertIn(word, c.exception.message, attrs)
        with self.assertRaises(L["ParamError"]) as c:
            self.lay(storeys=1, stair=None, setback=[{"storey": 1, "sides": [0], "by": 3.0}])
        self.assertIn("names storey 1", c.exception.message)
        ell = [(0, 0), (12600, 0), (12600, 6600), (6600, 6600), (6600, 12600), (0, 12600)]
        rows = kit()
        with self.assertRaises(L["ParamError"]) as c:
            L["plan"]("b", ell, {"storeys": 2, "storey_h": 3.3, "kit": "k", "stair": {"side": 5, "at": 1.8},
                                 "setback": [{"storey": 1, "sides": [0], "by": 3.0}]}, 10.0, None, None,
                      kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertIn("rectangle only", c.exception.message)

    def test_a_row_passes_a_set_back_only_to_the_lots_that_have_the_storey(self):
        lots = [{"bays": 4, "storeys": 3}, {"bays": 4, "storeys": 2}]
        sb = [{"storey": 2, "sides": [0], "by": 3.0}]
        got, _ = plan([("s", [(18000, 16000)], dict(SOUTH, lots=lots, to=[43.2, 16], setback=sb))])
        self.assertEqual(got[0]["attrs"]["setback"], sb)
        self.assertNotIn("setback", got[1]["attrs"])
        ov = [{"storey": 1, "sides": [0], "by": 1.5}]
        got, _ = plan([("s", [(18000, 16000)], dict(SOUTH, lots=lots, to=[43.2, 16], overhang=ov))])
        self.assertEqual((got[0]["attrs"]["overhang"], got[1]["attrs"]["overhang"]), (ov, ov))


class EllRoof(unittest.TestCase):
    """lay.py rule 10, "Over an L": a hip at the outer corner, a valley at the inside one."""

    # The village kit's roof: 1.5 m slopes of 3.3 m span, for wings 6.6 m deep; walls 0.3 m.
    KIT = {"roof_slope": {"id": "slope", "pitch": 1500, "span": 3300}, "roof_gable": {"id": "gable", "span": 3300},
           "roof_hip": {"id": "hip", "span": 3300}, "roof_valley": {"id": "valley", "span": 3300, "reach": 1200}}
    # An L of two wings 6.6 m deep: 12.6 m along the south, 9.6 m along the west; the inside corner
    # is (6600, 6600), the outer one (0, 0).
    ELL = [(0, 0), (12600, 0), (12600, 6600), (6600, 6600), (6600, 9600), (0, 9600)]

    def roof(self, ring=None, **kit):
        return L["ell_plan"](ring or self.ELL, dict(self.KIT, **kit), 300, "h_roof")

    def test_the_four_eaves_each_hold_whole_slopes(self):
        """South (outer): 12.6 m less the hip's span and the gable's wall is 9 m, six pieces from
        3.3 m. West (outer): 9.6 m less the same is 6 m, four pieces ending 3.3 m short of the
        outer corner. The inner eaves: 6 m and 3 m less the wall and the valley's 1.2 m reach
        are 4.5 m and 1.5 m, three pieces and one."""
        runs, insts = self.roof()
        got = {r["key"]: (r["count"], r["face"]) for r in runs}
        self.assertEqual(got, {"h_roof_e0": (6, 180.0), "h_roof_e5": (4, 270.0), "h_roof_e2": (3, 0.0),
                               "h_roof_e3": (1, 90.0)})
        total = 12600 + 6600 + 6000 + 3000 + 6600 + 9600
        span = {r["key"]: [round(v * total) for v in r["span"]] for r in runs}
        self.assertEqual(span["h_roof_e0"], [3300, 12300])                                  # past the hip, to the gable's wall
        self.assertEqual(span["h_roof_e5"], [total - 9600 + 300, total - 3300])             # from the gable's wall to the hip
        self.assertEqual(span["h_roof_e2"], [12600 + 6600 + 300, 12600 + 6600 + 6000 - 1200])  # to the valley's reach
        self.assertEqual(span["h_roof_e3"], [12600 + 6600 + 6000 + 1200, 12600 + 6600 + 6000 + 3000 - 300])
        self.assertTrue(all(r["offset"] == 1.65 and r["ends"] == "half" and r["ref"] == "h_roof" for r in runs))

    def test_the_hip_stands_on_the_outer_corner_and_the_valley_on_the_inside_one(self):
        _, insts = self.roof()
        at = {i["key"]: (i["asset"], i["at"], i["face"]) for i in insts}
        # The hip's middle is half a span in from both outer eaves; its front is out of the south
        # side, which starts at the outer corner, so its +x eave lies on the west side.
        self.assertEqual(at["h_roof_hip"], ("hip", (1650, 1650), 180.0))
        # The valley is placed by the inside corner, its front out of the side that follows it
        # (the west wing's inner eave, which faces east).
        self.assertEqual(at["h_roof_valley"], ("valley", (6600, 6600), 90.0))
        gables = sorted(k for k in at if "_g" in k)
        self.assertEqual(gables, ["h_roof_g1_0", "h_roof_g1_1", "h_roof_g4_0", "h_roof_g4_1"])
        self.assertEqual(at["h_roof_g1_0"][1:], ((12450, 1650), 90.0))                      # the east end: low at the south eave
        self.assertEqual(at["h_roof_g1_1"][1:], ((12450, 4950), 270.0))
        self.assertEqual(at["h_roof_g4_0"][1:], ((4950, 9450), 0.0))                        # the north end
        self.assertTrue(all(i["surface"] == "h_roof.garden" for i in insts))

    def test_the_same_roof_whichever_vertex_the_ring_starts_at_and_the_other_hand(self):
        for k in range(6):
            ring = self.ELL[k:] + self.ELL[:k]
            runs, insts = self.roof(ring)
            self.assertEqual(sorted(r["count"] for r in runs), [1, 3, 4, 6], k)
            at = {i["asset"]: i["at"] for i in insts if i["asset"] in ("hip", "valley")}
            self.assertEqual(at, {"hip": (1650, 1650), "valley": (6600, 6600)}, k)
        # The mirror L, its long wing to the north: the inside corner follows the other way round.
        other = [(0, 0), (6600, 0), (6600, 3000), (12600, 3000), (12600, 9600), (0, 9600)]
        runs, insts = self.roof(other)
        at = {i["asset"]: (i["at"], i["face"]) for i in insts if i["asset"] in ("hip", "valley")}
        self.assertEqual(at["valley"][0], (6600, 3000))
        self.assertEqual(at["hip"][0], (1650, 9600 - 1650))
        self.assertEqual(sorted(r["count"] for r in runs), [1, 3, 4, 6])

    def test_what_is_refused_says_why(self):
        deep = [(0, 0), (12600, 0), (12600, 8100), (6600, 8100), (6600, 9600), (0, 9600)]   # one wing 8.1 m deep
        off = [(0, 0), (13100, 0), (13100, 6600), (6600, 6600), (6600, 9600), (0, 9600)]    # the south 0.5 m longer
        for ring, kit, word in ((deep, {}, "both wings must be that deep"),
                                (off, {}, "side 2 of the L, an inner eave, holds 5 m"),      # the first of the two eaves it lengthens
                                (None, {"roof_valley": {"id": "valley", "span": 3300, "reach": 300}}, "an inner eave, holds"),
                                (None, {"roof_slope": {"id": "slope", "pitch": 0, "span": 3300}}, "states no length along the eave"),
                                (None, {"roof_hip": None}, "has no `role:roof_hip`"),
                                (None, {"roof_valley": {"id": "valley", "span": 4500, "reach": 1200}}, "the valley piece is made for a span of 4.5 m")):
            got, why = self.roof(ring, **kit)
            self.assertIsNone(got, word)
            self.assertIn(word, why)
        self.assertIn("0.5 m shorter or 1 m longer", self.roof(off)[1])

    def test_a_wing_shorter_than_what_stands_on_it_is_told_the_length_it_needs(self):
        """An inner side of 0.9 m, where the valley's reach and the gable's wall take 1.5 m: no
        "shorter" length is offered, since none is an L."""
        stub = [(0, 0), (12600, 0), (12600, 6600), (6600, 6600), (6600, 7500), (0, 7500)]
        got, why = self.roof(stub)
        self.assertIsNone(got)
        self.assertIn("side 3 of the L, an inner eave, is 0.9 m long", why)
        self.assertIn("take 1.5 m of it", why)
        self.assertNotIn("shorter", why)

    def test_an_eave_with_no_room_for_a_slope_gets_no_run(self):
        """An inner side of exactly the reach and the wall: the valley's ear meets the gable's
        wall, and no run of no pieces is returned for that side."""
        snug = [(0, 0), (12600, 0), (12600, 6600), (6600, 6600), (6600, 8100), (0, 8100)]
        runs, insts = self.roof(snug)
        self.assertEqual(sorted(r["key"] for r in runs), ["h_roof_e0", "h_roof_e2", "h_roof_e5"])
        self.assertTrue(all(r["count"] > 0 for r in runs))
        self.assertEqual(len(insts), 6)

    def test_an_l_must_be_square_at_every_corner(self):
        skew = [(0, 0), (12600, 0), (12600, 6600), (6600, 6600), (6900, 9600), (0, 9600)]
        got, why = self.roof(skew)
        self.assertIsNone(got)
        self.assertIn("neither a rectangle nor an L", why)

    def test_a_building_asks_for_it_as_any_gable(self):
        def row_(id_, role, size, high=2700, tags=(), **params):
            return row(id_, role, size, high, tags, **params)
        rows = [row_("frame", "wall", (3000, 300), pitch=3), row_("half", "filler", (1500, 300), pitch=1.5),
                row_("post", "corner", (300, 300)), row_("door", "door", (1500, 300), pitch=1.5),
                {"id": "corner_in", "tags": ["role:corner_in"], "bounds": [-1350, -150, 0, 150, 1350, 2700],
                 "params": {"reach": 1.2}, "shared": {}},
                row_("slope", "roof_slope", (1500, 3300), 3000, pitch=1.5, span=3.3),
                row_("gable", "roof_gable", (3300, 300), 3000, span=3.3),
                row_("hip", "roof_hip", (3300, 3300), 3000, span=3.3),
                {"id": "valley", "tags": ["role:roof_valley"], "bounds": [-1200, -3300, 0, 3300, 1200, 3000],
                 "params": {"span": 3.3, "reach": 1.2}, "shared": {}}]
        a = {"storeys": 1, "storey_h": 2.7, "plinth": 0.3, "porch": 2, "walk_out": 3, "kit": "k", "roof": "gable",
             "doors": [{"storey": 0, "side": 0, "at": 4.05}]}
        out = L["plan"]("h", self.ELL, a, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h))
        self.assertFalse([n for n in out["notes"] if n["code"] == "roof_none"])
        self.assertEqual(out["open"], [])                                                   # the L's walls close too
        self.assertEqual(sorted(i["asset"] for i in out["instances"] if "_roof_" in i["key"]),
                         ["gable"] * 4 + ["hip", "valley"])
        self.assertEqual(sum(r["count"] for r in out["runs"] if r["asset"] == "slope"), 14)


if __name__ == "__main__":
    unittest.main()
