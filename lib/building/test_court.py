"""Tests for lay.py rules 17 and 18 (a hipped roof, eaves, roof pieces by span; a wall on the
ground), the floor of rule 15, what check.py holds a roof under eaves to, and court.py.

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/building -p 'test_*.py'

The kits are examples/court's (packs/timber.toml), as `pack.query` gives their rows: `timber`,
laid by posts on a 3.0 m bay with posts of 0.3 m and a roof 1.35 m beyond them, and `yard`, a
wall of 3.0 m bays 0.3 m thick with a gate.
"""

import json
import math
import os
import runpy
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
L = runpy.run_path(os.path.join(HERE, "lay.py"))
C = runpy.run_path(os.path.join(HERE, "check.py"))
K = runpy.run_path(os.path.join(HERE, "court.py"))
P = C["params_of"]({})
PE, KE = L["ParamError"], K["ParamError"]

SHARED = {"unit": 0.15, "wall": 0.15, "storey": 3.6, "door": 1.5, "base": 0.6, "eaves": 1.35}
YARD = {"wall": 0.3, "storey": 2.4, "door": 2.4}


def row(id_, roles, size, high=3600, tags=(), z0=0, shared=None, taxon="timber", **params):
    """A pack row: a box `size` (length, thickness) mm in plan, from `z0` up `high`."""
    w, t = size
    roles = [roles] if isinstance(roles, str) else roles
    return {"id": id_, "tags": [f"role:{r}" for r in roles] + list(tags), "taxon": taxon,
            "bounds": [-w // 2, -t // 2, z0, w - w // 2, t - t // 2, z0 + high], "params": dict(params),
            "shared": SHARED if shared is None else shared}


def timber(shared=None, swap=None, drop=()):
    """The timber kit's rows; `swap` replaces rows by id, `drop` leaves ids out."""
    swap = swap or {}
    rows = [row("post", ["post", "corner"], (300, 300)),
            row("panel", "wall", (2700, 150), pitch=3.0),
            row("panel_lattice", "wall", (2700, 150), tags=("variant:lattice",), pitch=3.0),
            row("panel_doors", "wall", (2700, 150), tags=("variant:doors",), pitch=3.0),
            row("door", "door", (2700, 150), pitch=3.0),
            row("tile", "floor", (1500, 1500), 100, pitch=1.5),
            row("roof_slope_6", "roof_slope", (3000, 6000), 2400, pitch=3.0, span=6.0),
            row("roof_hip_6", "roof_hip", (6000, 6000), 2400, span=6.0),
            row("roof_slope_45", "roof_slope", (3000, 4500), 1800, pitch=3.0, span=4.5),
            row("roof_hip_45", "roof_hip", (4500, 4500), 1800, span=4.5),
            row("base", "wall", (3000, 150), 600, ("storey:base",), pitch=3.0),
            row("base_corner", "corner", (150, 150), 600, ("storey:base",)),
            row("veranda_post", ["post", "corner"], (300, 300), tags=("storey:colonnade",)),
            row("beam", "wall", (2700, 150), 450, ("storey:colonnade",), z0=3150, pitch=3.0)]
    rows = [swap.get(r["id"], r) for r in rows if r["id"] not in drop]
    return rows if shared is None else [dict(r, shared=shared) for r in rows]


def yard_kit():
    return [row("yard_wall", "wall", (3000, 300), 2400, shared=YARD, taxon="yard", pitch=3.0),
            row("yard_corner", "corner", (300, 300), 2400, shared=YARD, taxon="yard"),
            row("yard_gate", "door", (3000, 300), 2400, shared=YARD, taxon="yard", pitch=3.0)]


HALL = [(0, 0), (15300, 0), (15300, 9300), (0, 9300)]          # five bays by three, and a post
WING = [(0, 0), (9300, 0), (9300, 6300), (0, 6300)]            # three bays by two
ATTRS = {"storeys": 1, "storey_h": 3.6, "plinth": 0.6, "porch": 2, "walk_out": 3, "kit": "timber", "roof": "hip"}


def lay(ring=HALL, rows=None, **attrs):
    rows = timber() if rows is None else rows
    foot = [(p[0], p[1], 10.0) for p in ring]
    return L["plan"]("t", ring, {**ATTRS, **attrs}, 10.0, None, None, kits=lambda w, h: L["kit_pieces"](rows, w, h),
                     foot=foot)


def grown(ring, by):
    (x0, y0), (x1, y1) = ring[0], ring[2]
    return [(x0 - by, y0 - by), (x1 + by, y0 - by), (x1 + by, y1 + by), (x0 - by, y1 + by)]


def roof_of(out):
    runs = {r["key"]: r for r in out["runs"] if "_roof_" in r["key"]}
    insts = {i["key"]: i for i in out["instances"] if "_roof_" in i["key"]}
    return runs, insts


def notes(out, code):
    return [n for n in out["notes"] if n["code"] == code]


class Hip(unittest.TestCase):
    """lay.py rule 17: a hipped roof on a rectangle."""

    def test_a_hip_on_each_corner_and_a_run_of_slopes_between_two(self):
        out = lay()
        runs, insts = roof_of(out)
        self.assertEqual(sorted(insts), ["t_roof_h0", "t_roof_h1", "t_roof_h2", "t_roof_h3"])
        self.assertEqual({i["asset"] for i in insts.values()}, {"roof_hip_6"})
        # the roof's outline is 18 by 12 m; each hip is 6 m square, in its corner
        self.assertEqual([insts[f"t_roof_h{k}"]["at"] for k in range(4)],
                         [(1650, 1650), (13650, 1650), (13650, 7650), (1650, 7650)])
        es = L["edges"](grown(HALL, 1350))
        self.assertEqual([insts[f"t_roof_h{k}"]["face"] for k in range(4)], [L["outward"](e) for e in es])
        self.assertTrue(all(i["surface"] == "t_roof.garden" for i in insts.values()))
        # the long sides hold 6 m of eave between their hips: two slopes of 3 m; the short sides none
        self.assertEqual(sorted(runs), ["t_roof_e0", "t_roof_e2"])
        for k in (0, 2):
            r = runs[f"t_roof_e{k}"]
            self.assertEqual((r["asset"], r["count"], r["ref"], r["offset"], r["face"]),
                             ("roof_slope_6", 2, "t_roof", 3.0, L["outward"](es[k])))
        self.assertEqual(runs["t_roof_e0"]["span"], [0.1, 0.2])            # 6 to 12 m of a 60 m outline
        self.assertEqual(notes(out, "roof_none"), [])

    def test_a_square_roof_is_four_hips(self):
        out = lay(ring=[(0, 0), (9300, 0), (9300, 9300), (0, 9300)])
        runs, insts = roof_of(out)
        self.assertEqual((sorted(runs), len(insts)), ([], 4))

    def test_an_eave_that_is_no_whole_number_of_slopes_is_refused_with_what_would_close(self):
        out = lay(ring=[(0, 0), (16300, 0), (16300, 9300), (0, 9300)])
        (n,) = notes(out, "roof_none")
        self.assertIn("side 0 holds 7 m of eave between its two hips, no whole number of 3 m slope pieces: a side "
                      "1 m shorter or 2 m longer would close", n["why"])
        self.assertTrue(n["flat"])

    def test_a_plan_that_is_no_rectangle_is_refused(self):
        ell = [(0, 0), (12000, 0), (12000, 6000), (6000, 6000), (6000, 12000), (0, 12000)]
        runs, why = L["hip_plan"](ell, L["kit_pieces"](timber()), "t_roof")
        self.assertIsNone(runs)
        self.assertIn("not a rectangle", why)

    def test_a_kit_with_no_hip_is_refused(self):
        out = lay(rows=timber(drop=("roof_hip_6", "roof_hip_45")))
        self.assertIn("no `role:roof_slope` and `role:roof_hip` pieces", notes(out, "roof_none")[0]["why"])

    def test_a_hip_of_another_span_than_the_slope_is_refused(self):
        rows = timber(drop=("roof_slope_45", "roof_hip_45", "roof_hip_6"))
        rows.append(row("roof_hip_6", "roof_hip", (5000, 5000), 2400, span=5.0))
        self.assertIn("the hip piece is made for a span of 5 m and the slope piece for 6 m",
                      notes(lay(rows=rows), "roof_none")[0]["why"])

    def test_any_other_word_for_a_roof_is_refused(self):
        with self.assertRaises(PE) as c:
            lay(roof="hipped")
        self.assertIn('"hip"', c.exception.hint)


class Eaves(unittest.TestCase):
    """lay.py rule 17: a roof that stands beyond the walls."""

    def test_the_roofs_region_is_the_outline_grown_by_the_kits_eaves(self):
        self.assertEqual(lay()["roof"]["region"], grown(HALL, 1350))

    def test_a_kit_that_states_no_eaves_roofs_the_outline(self):
        plain = {k: v for k, v in SHARED.items() if k != "eaves"}
        out = lay(ring=[(0, 0), (15000, 0), (15000, 12000), (0, 12000)], rows=timber(shared=plain))
        self.assertEqual(out["roof"]["region"], [(0, 0), (15000, 0), (15000, 12000), (0, 12000)])
        self.assertEqual(roof_of(out)[1]["t_roof_h0"]["at"], (3000, 3000))

    def test_a_roof_that_is_not_laid_says_the_eaves_it_was_sized_by(self):
        out = lay(rows=timber(shared=dict(SHARED, eaves=0.9)))
        (n,) = notes(out, "roof_none")
        self.assertIn("the roof is 11.1 m deep and the kit's slope pieces are made for roofs 9 and 12 m deep", n["why"])
        self.assertIn("the roof stands 0.9 m beyond the outline on every side (`shared.eaves`)", n["why"])
        # the flat roof that stands in its place has no eaves
        self.assertEqual(out["roof"]["region"], HALL)

    def test_a_flat_roof_has_no_eaves_and_is_whole_tiles_over_posts(self):
        out = lay(roof="flat")
        self.assertEqual(out["roof"]["region"], HALL)
        self.assertEqual(sum(1 for i in out["instances"] if i["key"].startswith("t_roof_f")), 10 * 6)
        self.assertEqual(notes(out, "slab_open"), [])

    GABLED = [row("slope", "roof_slope", (3000, 6000), 2400, pitch=3.0, span=6.0),
              row("gable", "roof_gable", (4650, 150), 2400, span=4.65)]

    def gabled(self, **swap):
        rows = timber(drop=("roof_slope_6", "roof_hip_6", "roof_slope_45", "roof_hip_45"))
        return rows + [swap.get(r["id"], r) for r in self.GABLED]

    def test_under_eaves_a_gables_slopes_run_the_roofs_whole_length(self):
        out = lay(rows=self.gabled(), roof="gable")
        runs, insts = roof_of(out)
        self.assertEqual(notes(out, "roof_none"), [])
        self.assertEqual({k: r["count"] for k, r in runs.items()}, {"t_roof_e0": 6, "t_roof_e2": 6})
        self.assertEqual(runs["t_roof_e0"]["span"], [0.0, 0.3])              # all 18 m of a 60 m outline

    def test_under_eaves_the_gable_halves_stand_over_the_wall(self):
        out = lay(rows=self.gabled(), roof="gable")
        _, insts = roof_of(out)
        # the east end of the roof is at x 16650; the wall's end line is the eaves in from it,
        # and the two halves span the wall, 9.3 m, from its two ends
        self.assertEqual(sorted(i["at"] for k, i in insts.items() if k.startswith("t_roof_g1")),
                         [(15225, 2325), (15225, 6975)])
        self.assertEqual(sorted(i["at"] for k, i in insts.items() if k.startswith("t_roof_g3")),
                         [(75, 2325), (75, 6975)])

    def test_a_gable_made_for_the_slopes_whole_span_does_not_fit_under_eaves(self):
        whole = row("gable", "roof_gable", (6000, 150), 2400, span=6.0)
        (n,) = notes(lay(rows=self.gabled(gable=whole), roof="gable"), "roof_none")
        self.assertIn("the gable piece is made for a span of 6 m, and under eaves of 1.35 m the wall it stands on is "
                      "9.3 m long: a gable piece for this roof has a span of 4.65 m", n["why"])

    def test_a_gable_under_eaves_that_is_no_whole_number_of_slopes(self):
        (n,) = notes(lay(ring=[(0, 0), (16300, 0), (16300, 9300), (0, 9300)], rows=self.gabled(), roof="gable"), "roof_none")
        self.assertIn("the roof is 19 m long with its eaves, no whole number of 3 m slope pieces: a building 1 m shorter "
                      "or 2 m longer would close", n["why"])

    def test_a_gable_under_eaves_over_an_l_is_not_built(self):
        ell = [(0, 0), (18300, 0), (18300, 9300), (9300, 9300), (9300, 18300), (0, 18300)]
        runs, why = L["gable_plan"](ell, L["kit_pieces"](self.gabled()), 150, "t_roof", 1350)
        self.assertIsNone(runs)
        self.assertIn("a gable under eaves over any other plan is not built", why)

    def test_with_no_eaves_a_gable_is_laid_as_it_was(self):
        plain = {k: v for k, v in SHARED.items() if k != "eaves"}
        rows = [dict(r, shared=plain) for r in self.gabled(gable=row("gable", "roof_gable", (6000, 150), 2400, span=6.0))]
        out = lay(ring=[(0, 0), (15300, 0), (15300, 12000), (0, 12000)], rows=rows, roof="gable")
        runs, insts = roof_of(out)
        self.assertEqual(runs["t_roof_e0"]["count"], 5)                      # 15.3 m less the two gables' 0.15 m
        self.assertEqual(sorted(i["at"] for k, i in insts.items() if k.startswith("t_roof_g1")),
                         [(15225, 3000), (15225, 9000)])


class BySpan(unittest.TestCase):
    """lay.py rule 17: of several roof pieces, those made for the roof's depth."""

    def test_a_hall_two_bays_deep_takes_the_smaller_pieces(self):
        runs, insts = roof_of(lay(ring=WING))
        self.assertEqual({i["asset"] for i in insts.values()}, {"roof_hip_45"})
        self.assertEqual({k: (r["asset"], r["count"]) for k, r in runs.items()},
                         {"t_roof_e0": ("roof_slope_45", 1), "t_roof_e2": ("roof_slope_45", 1)})
        self.assertEqual(insts["t_roof_h0"]["at"], (900, 900))

    def test_a_depth_no_piece_is_made_for_names_the_depths_the_kit_roofs(self):
        (n,) = notes(lay(ring=[(0, 0), (9300, 0), (9300, 3300), (0, 3300)]), "roof_none")
        self.assertIn("the roof is 6 m deep and the kit's slope pieces are made for roofs 9 and 12 m deep "
                      "(twice their spans)", n["why"])

    def test_the_kit_keeps_every_roof_piece_and_one_for_each_role_as_before(self):
        kit = L["kit_pieces"](timber())
        self.assertEqual([x["id"] for x in kit["roofs"]["roof_slope"]], ["roof_slope_45", "roof_slope_6"])
        self.assertEqual(kit["roof_slope"]["id"], "roof_slope_45")           # the first id bytewise
        chosen, why = L["roof_for"](kit, grown(HALL, 1350))
        self.assertEqual((chosen["roof_slope"]["id"], chosen["roof_hip"]["id"], why), ("roof_slope_6", "roof_hip_6", None))

    def test_a_kit_of_one_span_is_left_to_the_roofs_own_rule(self):
        rows = timber(drop=("roof_slope_45", "roof_hip_45"))
        kit = L["kit_pieces"](rows)
        self.assertIs(L["roof_for"](kit, WING)[0], kit)
        (n,) = notes(lay(ring=WING, rows=rows), "roof_none")
        self.assertIn("the roof is 9 m deep and the kit's slope piece is made for 12 m (twice its span)", n["why"])


class Floor(unittest.TestCase):
    """lay.py rule 15, "The floor": tiles to the middle of the line of posts."""

    def tiles(self, out, head):
        return sorted(i["at"] for i in out["instances"] if i["key"].startswith(head + "_f"))

    def test_a_storey_walled_by_posts_is_tiled_from_post_middle_to_post_middle(self):
        out = lay()
        at = self.tiles(out, "t_s0")
        self.assertEqual((len(at), at[0], at[-1]), (10 * 6, (900, 900), (14400, 8400)))
        self.assertEqual(notes(out, "slab_open"), [])

    def test_a_panel_of_any_thickness_leaves_the_tiles_where_they_are(self):
        thick = timber(shared=dict(SHARED, wall=0.3), swap={
            "panel": row("panel", "wall", (2700, 300), pitch=3.0), "door": row("door", "door", (2700, 300), pitch=3.0),
            "panel_lattice": row("panel_lattice", "wall", (2700, 300), tags=("variant:lattice",), pitch=3.0),
            "panel_doors": row("panel_doors", "wall", (2700, 300), tags=("variant:doors",), pitch=3.0)})
        self.assertEqual(len(self.tiles(lay(rows=thick), "t_s0")), 60)
        # and a flat roof over them is whole tiles too, to the same line
        out = lay(rows=thick, roof="flat")
        self.assertEqual((len(self.tiles(out, "t_roof")), notes(out, "slab_open")), (60, []))

    def test_a_veranda_one_bay_deep_is_whole_tiles_and_so_is_the_room_behind_it(self):
        out = lay(colonnade=[{"sides": [0], "depth": 3.0}], doors=[{"storey": 0, "side": 0, "at": 7.65}])
        porch, room = self.tiles(out, "t_platform"), self.tiles(out, "t_s0")
        self.assertEqual((len(porch), porch[0], porch[-1]), (10 * 2, (900, 900), (14400, 2400)))
        self.assertEqual((len(room), room[0], room[-1]), (10 * 4, (900, 3900), (14400, 8400)))
        self.assertEqual(notes(out, "slab_open"), [])


class Fit(unittest.TestCase):
    """lay.py rule 6: a stretch a fraction of a millimetre short of whole pieces closes."""

    def test_a_stretch_just_short_of_whole_pieces_closes(self):
        self.assertEqual(L["fit"](5999.62, 3000, 0), ((2, 0), None))
        self.assertEqual(L["fit"](6004.0, 3000, 0), ((2, 0), None))
        self.assertEqual(L["fit"](5999.62, 3000, 1500), ((2, 0), None))

    def test_a_stretch_more_than_the_tolerance_short_does_not(self):
        self.assertEqual(L["fit"](5990.0, 3000, 0), (None, (3000, 6000)))


class Yard(unittest.TestCase):
    """lay.py rule 18: a building of no storeys is its walls on the ground."""

    RING = [(0, 0), (39600, 0), (39600, 36600), (0, 36600)]              # thirteen bays by twelve
    A = {"storeys": 0, "storey_h": 2.4, "kit": "yard", "doors": [{"storey": 0, "side": 0, "at": 19.8}]}

    def lay(self, **attrs):
        rows = yard_kit()
        return L["plan"]("y", self.RING, {**self.A, **attrs}, 10.0, None, None,
                         kits=lambda w, h: L["kit_pieces"](rows, w, h), foot=[])

    def test_it_has_no_storey_no_floor_and_no_path(self):
        out = self.lay()
        for key in ("storeys", "rooms", "flights", "doorways", "footpaths", "footings", "terraces"):
            self.assertEqual(out[key], [], key)
        self.assertIsNone(out["roof"])
        self.assertIsNone(out["platform"])
        self.assertEqual(out["yard"], {"key": "y_yard", "region": self.RING})

    def test_its_walls_stand_round_the_yard_on_the_terrain(self):
        out = self.lay()
        self.assertEqual({r["ref"] for r in out["runs"]}, {"y_yard"})
        # the front is two stretches of six bays either side of the gate; the others 12, 13 and 12 bays
        self.assertEqual({r["key"]: r["count"] for r in out["runs"]},
                         {"y_yard_w0_0": 6, "y_yard_w0_1": 6, "y_yard_w1_0": 12, "y_yard_w2_0": 13, "y_yard_w3_0": 12})
        self.assertTrue(all(i["surface"] is None for i in out["instances"]))
        self.assertEqual(out["open"], [])

    def test_the_gate_is_the_kits_door_piece_in_a_gap(self):
        out = self.lay()
        (gate,) = [i for i in out["instances"] if i["asset"] == "yard_gate"]
        self.assertEqual((gate["key"], gate["at"]), ("y_yard_door0", (19800, 150)))
        self.assertEqual(sum(1 for i in out["instances"] if i["asset"] == "yard_corner"), 4)

    def test_what_a_building_of_no_storeys_does_not_take(self):
        for key, v in (("roof", "hip"), ("stair", {"side": 2, "at": 3}), ("rooms", [{"storey": 0, "shares": [1, 1]}]),
                       ("colonnade", [{"sides": [0], "depth": 3}]), ("setback", [{"storey": 0, "sides": [0], "by": 1}])):
            with self.assertRaises(PE) as c:
                self.lay(**{key: v})
            self.assertEqual(c.exception.key, key)
            self.assertIn(f"takes no `{key}`", c.exception.message)
        with self.assertRaises(PE) as c:
            L["plan"]("y", self.RING, {"storeys": 0}, 10.0, None, None)
        self.assertIn("has no kit with a bay", c.exception.message)
        with self.assertRaises(PE) as c:
            self.lay(doors=[{"storey": 1, "side": 0, "at": 19.8}])
        self.assertIn("the building has none", c.exception.message)

    def test_a_side_that_does_not_close_is_said_as_on_any_wall(self):
        out = L["plan"]("y", [(0, 0), (39600, 0), (39600, 37000), (0, 37000)], self.A, 10.0, None, None,
                        kits=lambda w, h: L["kit_pieces"](yard_kit(), w, h), foot=[])
        self.assertEqual(sorted(o["side"] for o in out["open"]), [1, 3])

    def test_a_storey_below_none_is_refused(self):
        with self.assertRaises(PE):
            self.lay(storeys=-1)


class FakeOut:
    def __init__(self):
        self.features, self.instances, self.tables = {}, {}, {}

    def feature(self, key, kind, at=None, path=None, region=None, **attrs):
        self.features[key] = dict(attrs, kind=kind, path=path, region=region)

    def instance(self, key, asset, at, face=None, **keys):
        self.instances[key] = dict(keys, asset=asset, at=at, face=face)

    def table(self, section, key, **keys):
        self.tables[(section, key)] = keys


class _FL(dict):
    def __len__(self):
        return len(self["geometry"])

    def rings(self, f):
        return [[self["coords"][f]]]


class FakeStep:
    """The step as `lay` and `court` read it: features by name with their attributes, level
    ground, the two kits' rows by taxon."""

    def __init__(self, feats, item=None, columns=None):
        self.params, self.grid, self.id, self.for_each_item = {}, None, "lay", item
        self.out, self.feats, self.said = FakeOut(), feats, []

    def features(self, select, columns=()):
        names = select if isinstance(select, list) else sorted(self.feats)
        fl = _FL({"name": names, "geometry": [2] * len(names), "coords": [[list(p) for p in self.feats[n][0]] for n in names]})
        for c in columns:
            fl[c], fl[c + "?"] = [], []
            for n in names:
                a = self.feats[n][1]
                v = a.get(c)
                fl[c].append(json.dumps(v) if isinstance(v, (list, dict)) else (v if c in a else 0))
                fl[c + "?"].append(1 if c in a else 0)
        return fl

    def fields(self, names, **kw):
        class A(list):
            name = "height"
        return [A()]

    def eval(self, formula, x, y):
        return [10.0] * len(x)

    def pack(self, select, columns=()):
        rows = yard_kit() if select == "@yard" else timber()
        return {"id": [r["id"] for r in rows], "tags": [json.dumps(r["tags"]) for r in rows],
                "bounds": [r["bounds"] for r in rows], "params": [json.dumps(r["params"]) for r in rows],
                "shared": [json.dumps(r["shared"]) for r in rows]}

    def _say(sev):
        def say(self, code, subject, message, hint, **kw):
            self.said.append((sev, code, subject, message, hint, kw.get("data")))
        return say
    error, warning, info = _say("error"), _say("warning"), _say("info")


class Said(unittest.TestCase):
    """What `lay` writes and says of a yard and of a hall (the step, through a fake of the SDK)."""

    def test_a_yard_is_written_as_a_feature_with_no_surface_and_pieces_on_the_terrain(self):
        s = FakeStep({"y": (Yard.RING, Yard.A)}, "y")
        L["generate"](s)
        self.assertEqual([x[:2] for x in s.said], [("info", "laid")])
        self.assertIn("0 storeys", s.said[0][3])
        self.assertEqual({k: v["kind"] for k, v in s.out.features.items()}, {"y_yard": "yard"})
        self.assertEqual(s.out.features["y_yard"]["region"], Yard.RING)
        self.assertTrue(s.out.instances)
        self.assertTrue(all("surface" not in i for i in s.out.instances.values()))
        self.assertEqual(s.out.tables[("run", "y_yard_w2_0")]["on"]["ref"], "y_yard")

    def test_a_hall_is_written_with_its_roof_beyond_its_walls(self):
        s = FakeStep({"t": (HALL, ATTRS)}, "t")
        L["generate"](s)
        self.assertEqual([x[:2] for x in s.said], [("info", "laid")])
        self.assertEqual(s.out.features["t_roof"]["region"], grown(HALL, 1350))
        self.assertEqual(s.out.instances["t_roof_h0"]["surface"], "t_roof.garden")

    def test_a_roof_no_piece_is_made_for_is_said_with_the_depths(self):
        s = FakeStep({"t": ([(0, 0), (9300, 0), (9300, 3300), (0, 3300)], ATTRS)}, "t")
        L["generate"](s)
        (w,) = [x for x in s.said if x[1] == "roof_none"]
        self.assertTrue(w[3].startswith("the hip roof is not laid: the roof is 6 m deep and the kit's slope pieces are "
                                        "made for roofs 9 and 12 m deep"))
        self.assertTrue(w[3].endswith("The flat roof is laid in its place"))


def judged(rows, features=(), meetings=(), pieces=()):
    return [(r[0], r[1], r[2], r[3]) for r in C["judge"](list(features), list(meetings), rows, P, list(pieces))
            if r[1] != "checked"]


class Checked(unittest.TestCase):
    """check.py: a kit of several roofs, a hip, and the eaves a kit states."""

    def test_the_timber_kit_and_the_yard_kit_say_nothing(self):
        self.assertEqual(judged(timber()), [])
        self.assertEqual(judged(yard_kit()), [])

    def test_a_slope_and_a_hip_are_a_roof_with_no_gable(self):
        self.assertFalse([s for s in judged(timber()) if s[1] == "kit_deck"])

    def test_a_slope_with_neither_end_is_said(self):
        said = [s for s in judged(timber(drop=("roof_hip_6", "roof_hip_45"))) if s[1] == "kit_deck"]
        self.assertEqual(sorted(s[2] for s in said), ["roof_slope_45", "roof_slope_6"])
        self.assertIn("no `role:roof_gable` piece", said[0][3])

    def test_a_slope_whose_ends_are_made_for_another_span_is_said(self):
        said = [s for s in judged(timber(drop=("roof_hip_45",))) if s[1] == "kit_deck"]
        self.assertEqual(len(said), 1)
        self.assertIn("`roof_hip_6` is made for a span of 6 m and `roof_slope_45` for 4.5 m: no roof takes both", said[0][3])

    def test_a_hip_is_one_span_square(self):
        rows = timber(swap={"roof_hip_6": row("roof_hip_6", "roof_hip", (6000, 5000), 2400, span=6.0)})
        said = [s for s in judged(rows) if s[1] == "kit_deck"]
        self.assertEqual([s[2] for s in said], ["roof_hip_6"])
        self.assertIn("measures 6 by 5 m in plan and states a span of 6 m: a hip is one span square", said[0][3])

    def test_under_eaves_a_gable_half_is_the_slopes_span_less_the_eaves(self):
        rows = timber(drop=("roof_hip_6", "roof_hip_45", "roof_slope_45"))
        ok = rows + [row("gable", "roof_gable", (4650, 150), 2400, span=4.65)]
        self.assertFalse([s for s in judged(ok) if s[1] == "kit_deck"])
        whole = rows + [row("gable", "roof_gable", (6000, 150), 2400, span=6.0)]
        said = [s for s in judged(whole) if s[1] == "kit_deck"]
        self.assertEqual(len(said), 1)
        self.assertIn("`gable` is made for a span of 6 m and `roof_slope_6` for 6 m", said[0][3])

    def test_stated_eaves_no_roof_piece_is_made_for_are_said_of_each_slope(self):
        said = [s for s in judged(timber(shared=dict(SHARED, eaves=0.9))) if s[1] == "kit_stated"]
        self.assertEqual([s[2] for s in said], ["roof_slope_45", "roof_slope_6"])
        self.assertIn("`roof_slope_6` has a span of 6 m, which under the stated eaves of 0.9 m roofs a building 10.2 m "
                      "deep: no building of the kit is that deep (whole bays of 3 m and one post of 0.3 m)", said[1][3])

    def test_stated_eaves_are_held_to_the_unit(self):
        said = [s for s in judged(timber(shared=dict(SHARED, eaves=1.425))) if s[1] == "kit_unit"]
        self.assertEqual(len(said), 1)
        self.assertIn("states `shared.eaves = 1.425`", said[0][3])

    def test_a_kit_of_whole_wall_pieces_under_eaves_is_held_to_its_corners(self):
        """Walls 0.3 m thick on a 3 m bay: a building is whole bays and 0.6 m, so a slope of span
        3.9 m under eaves of 0.6 m roofs 6.6 m, two bays; one of span 4 m roofs 6.8 m, none."""
        sh = {"wall": 0.3, "storey": 3.0, "eaves": 0.6}

        def kit(span):
            return [row("wall", "wall", (3000, 300), 3000, shared=sh, pitch=3.0),
                    row("corner", "corner", (300, 300), 3000, shared=sh),
                    row("slope", "roof_slope", (3000, int(span * 1000)), 1500, shared=sh, pitch=3.0, span=span),
                    row("hip", "roof_hip", (int(span * 1000), int(span * 1000)), 1500, shared=sh, span=span)]
        self.assertFalse([s for s in judged(kit(3.9)) if s[1] == "kit_stated"])
        said = [s for s in judged(kit(4.0)) if s[1] == "kit_stated"]
        self.assertEqual(len(said), 1)
        self.assertIn("roofs a building 6.8 m deep: no building of the kit is that deep (whole pieces of 3 m and two "
                      "corners of 0.3 m)", said[0][3])


def facts(taxon):
    return K["kit_facts"](yard_kit() if taxon == "yard" else timber(), taxon)


COURT = {"to": [32, 20], "kit": "timber", "storey_h": 3.6, "plinth": 0.6, "roof": "hip",
         "yard": {"kit": "yard", "bays": [13, 12]},
         "hall": {"bays": [5, 3], "from": 24, "veranda": 1, "fill": [{"sides": [0], "pattern": ["doors"]}]},
         "wings": [{"bays": [3, 2], "from": 9, "off": 9,
                    "fill": [{"sides": [0], "pattern": ["lattice"]}, {"sides": [1], "pattern": ["doors"]}]}]}
GATE = (32000, 10000)


def court(at=GATE, **attrs):
    a = {**COURT, **attrs}
    b, n = K["plan_court"]("c", at, {k: v for k, v in a.items() if v is not None}, facts)
    return {x["key"]: x for x in b}, n


class Court(unittest.TestCase):
    """court.py: a yard, a hall on the axis and wings in pairs."""

    def test_what_a_court_sizes_by(self):
        t = facts("timber")
        self.assertEqual((t["posts"], t["bay"], t["ends"], t["door"], t["eaves"], t["storey"], t["wall"]),
                         (True, 3000, 300, 1, 1350, 3600, 300))
        y = facts("yard")
        self.assertEqual((y["posts"], y["bay"], y["ends"], y["door"], y["eaves"], y["storey"], y["wall"]),
                         (False, 3000, 600, 1, 0, 2400, 300))
        self.assertEqual((K["side"](5, t), K["side"](13, y)), (15300, 39600))
        # a corner that states its reach sizes a side of whole wall pieces
        pier = [dict(r, params={"reach": 0.45}) if r["id"] == "yard_corner" else r for r in yard_kit()]
        self.assertEqual(K["kit_facts"](pier, "yard")["ends"], 900)

    def test_a_kit_with_no_bay_or_a_door_of_no_whole_bays_is_refused(self):
        with self.assertRaises(KE) as c:
            K["kit_facts"]([], "none")
        self.assertIn("has no `role:wall` piece", c.exception.message)
        rows = yard_kit()[:2] + [row("yard_gate", "door", (4000, 300), 2400, shared=YARD, taxon="yard", pitch=4.0)]
        with self.assertRaises(KE) as c:
            K["kit_facts"](rows, "yard")
        self.assertIn("takes 4 m of wall, no whole number of its 3 m bays", c.exception.message)

    def test_the_buildings_stand_about_the_axis(self):
        b, n = court()
        self.assertEqual(n, [])
        self.assertEqual(sorted(b), ["c_hall", "c_w0_l", "c_w0_r", "c_yard"])
        self.assertEqual(b["c_yard"]["ring"], [(12200, 10000), (51800, 10000), (51800, 46600), (12200, 46600)])
        self.assertEqual(b["c_hall"]["ring"], [(24350, 34000), (39650, 34000), (39650, 43300), (24350, 43300)])
        self.assertEqual(b["c_w0_l"]["ring"], [(23000, 19000), (23000, 28300), (16700, 28300), (16700, 19000)])
        self.assertEqual(b["c_w0_r"]["ring"], [(41000, 28300), (41000, 19000), (47300, 19000), (47300, 28300)])
        for x in b.values():                     # counter-clockwise, as `lay` reads an outline
            self.assertGreater(L["area2"](x["ring"]), 0)

    def test_the_yard_is_a_building_of_no_storeys_with_its_gate_on_the_axis(self):
        a = court()[0]["c_yard"]["attrs"]
        self.assertEqual(a, {"kit": "yard", "storey_h": 2.4, "storeys": 0, "doors": [{"storey": 0, "side": 0, "at": 19.8}]})

    def test_a_halls_door_is_on_its_fronts_middle_and_its_veranda_across_its_front(self):
        b, _ = court()
        a = b["c_hall"]["attrs"]
        self.assertEqual(a["doors"], [{"storey": 0, "side": 0, "at": 7.65}])
        self.assertEqual(a["colonnade"], [{"sides": [0], "depth": 3.0}])
        self.assertEqual((a["kit"], a["storey_h"], a["plinth"], a["roof"], a["storeys"]), ("timber", 3.6, 0.6, "hip", 1))
        self.assertEqual(a["fill"], [{"sides": [0], "pattern": ["doors"]}])
        self.assertNotIn("colonnade", b["c_w0_l"]["attrs"])
        self.assertEqual(b["c_w0_l"]["attrs"]["doors"], [{"storey": 0, "side": 0, "at": 4.65}])

    def test_a_wing_is_written_once_and_laid_twice_as_mirror_images(self):
        b, _ = court()
        left, right = b["c_w0_l"], b["c_w0_r"]
        self.assertEqual(sorted((2 * GATE[0] - x, y) for x, y in left["ring"]), sorted(right["ring"]))
        # side 0 of each is its front, the side nearest the axis
        for x in (left, right):
            (ax, _), (bx, _) = x["ring"][0], x["ring"][1]
            self.assertEqual(ax, bx)
            self.assertEqual(abs(ax - GATE[0]), 9000)
        # the left one's side 1 is its far end; the right one's its near end
        self.assertEqual((left["ring"][1][1], right["ring"][1][1]), (28300, 19000))
        self.assertEqual(left["attrs"]["fill"], [{"sides": [0], "pattern": ["lattice"], "mirror": True},
                                                 {"sides": [1], "pattern": ["doors"], "mirror": True}])
        self.assertEqual(right["attrs"]["fill"], [{"sides": [0], "pattern": ["lattice"], "mirror": True},
                                                  {"sides": [3], "pattern": ["doors"], "mirror": True}])

    def test_the_twins_are_laid_as_images_piece_for_piece(self):
        """Through `lay`: every piece of the left wing has its image in the right one."""
        b, _ = court()
        got = []
        for key in ("c_w0_l", "c_w0_r"):
            out = L["plan"](key, b[key]["ring"], dict(b[key]["attrs"], porch=2, walk_out=3), 10.0, None, None,
                            kits=lambda w, h: L["kit_pieces"](timber(), w, h), foot=[])
            self.assertEqual(notes(out, "roof_none") + out["open"], [])
            got.append(sorted((i["asset"], i["at"]) for i in out["instances"]))
        self.assertEqual(sorted((a, (2 * GATE[0] - x, y)) for a, (x, y) in got[0]), got[1])

    def test_a_front_of_an_even_number_of_bays_has_no_bay_on_its_middle(self):
        with self.assertRaises(KE) as c:
            court(hall={"bays": [4, 3], "from": 24})
        self.assertEqual(c.exception.key, "hall")
        self.assertIn("the hall is 4 bays across, and its door stands on its middle, where a front of 4 bays has a post",
                      c.exception.message)
        self.assertEqual(c.exception.hint, "make it 3 or 5 bays across")
        with self.assertRaises(KE) as c:
            court(yard={"kit": "yard", "bays": [12, 12]})
        self.assertIn("the yard's front is 12 bays across", c.exception.message)
        self.assertIn("a joint between two bays", c.exception.message)
        with self.assertRaises(KE) as c:
            court(wings=[{"bays": [2, 2], "from": 9, "off": 9}])
        self.assertIn("wing 0 is 2 bays across", c.exception.message)

    def test_eaves_that_meet_are_said_and_both_are_returned(self):
        b, n = court(wings=[{"bays": [3, 2], "from": 13, "off": 9}])
        self.assertEqual([(x["key"], x["other"], x["by"], x["eaves"], x["dropped"]) for x in n],
                         [("c_w0_l", "c_hall", 1000.0, True, False), ("c_w0_r", "c_hall", 1000.0, True, False)])
        self.assertEqual(len(b), 4)

    def test_eaves_that_only_touch_are_not_said(self):
        self.assertEqual(court(wings=[{"bays": [3, 2], "from": 12, "off": 9}])[1], [])

    def test_two_outlines_in_one_place_are_said_and_the_later_is_not_returned(self):
        b, n = court(wings=[{"bays": [3, 2], "from": 16, "off": 5}])
        self.assertEqual(sorted(b), ["c_hall", "c_yard"])
        self.assertEqual([(x["key"], x["other"], x["by"], x["eaves"], x["dropped"]) for x in n],
                         [("c_w0_l", "c_hall", 1300.0, False, True), ("c_w0_r", "c_hall", 1300.0, False, True)])

    def test_a_hall_or_its_eaves_past_the_yards_wall(self):
        b, n = court(wings=[{"bays": [3, 2], "from": 9, "off": 12}])          # the eaves, by 0.15 m
        self.assertEqual([(x["key"], x["other"], round(x["by"]), x["eaves"], x["wall"], x["dropped"]) for x in n],
                         [("c_w0_l", "c_yard", 150, True, True, False), ("c_w0_r", "c_yard", 150, True, True, False)])
        self.assertEqual(len(b), 4)
        b, n = court(wings=[{"bays": [3, 2], "from": 9, "off": 13.5}])        # the walls, by 0.3 m
        self.assertEqual([(x["key"], round(x["by"]), x["eaves"], x["dropped"]) for x in n],
                         [("c_w0_l", 300, False, True), ("c_w0_r", 300, False, True)])
        self.assertEqual(sorted(b), ["c_hall", "c_yard"])

    def test_a_court_turned_off_the_axes_keeps_its_lengths_and_its_mirror(self):
        b, n = court(to=[35, 20])
        self.assertEqual(n, [])
        v = (3 / math.hypot(3, 10), 10 / math.hypot(3, 10))

        def frame(p):                            # (right of the way in, along the axis), mm
            dx, dy = p[0] - GATE[0], p[1] - GATE[1]
            return (dx * v[1] - dy * v[0], dx * v[0] + dy * v[1])
        sides = [math.dist(p, q) for p, q in zip(b["c_w0_l"]["ring"], b["c_w0_l"]["ring"][1:] + b["c_w0_l"]["ring"][:1])]
        for got, want in zip(sides, (9300, 6300, 9300, 6300)):
            self.assertLess(abs(got - want), 1.5)
        left = [frame(p) for p in b["c_w0_l"]["ring"]]
        right = [(-u, w) for u, w in (frame(p) for p in b["c_w0_r"]["ring"])]
        for a in left:                           # each corner has its image, to the rounding of two points
            self.assertLess(min(math.dist(a, q) for q in right), 1.5)

    def test_a_court_may_have_no_yard_no_hall_or_no_wings(self):
        self.assertEqual(sorted(court(yard=None)[0]), ["c_hall", "c_w0_l", "c_w0_r"])
        self.assertEqual(sorted(court(hall=None, wings=None)[0]), ["c_yard"])
        self.assertEqual(sorted(court(wings=[])[0]), ["c_hall", "c_yard"])

    def test_what_a_court_is_refused_for(self):
        for attrs, key in (({"to": [32, 10]}, "to"), ({"to": "north"}, "to"), ({"kit": ""}, "kit"),
                           ({"yard": {"kit": "yard", "bays": [13]}}, "yard"), ({"yard": {"bays": [13, 12]}}, "yard"),
                           ({"yard": {"kit": "yard", "bays": [13, 12], "storey_h": 0.2}}, "yard"),
                           ({"hall": {"bays": [5, 3]}}, "hall"), ({"hall": {"bays": [5, 3], "from": 24, "veranda": 3}}, "hall"),
                           ({"hall": {"bays": [5.5, 3], "from": 24}}, "hall"), ({"hall": {"bays": [5, 3], "from": 24, "at": 1}}, "hall"),
                           ({"wings": [{"bays": [3, 2], "from": 9}]}, "wings"), ({"wings": "two"}, "wings"),
                           ({"wings": [{"bays": [3, 2], "from": 9, "off": -1}]}, "wings")):
            with self.assertRaises(KE, msg=str(attrs)) as c:
                court(**attrs)
            self.assertEqual(c.exception.key, key, str(attrs))

    def test_whole_numbers_read_through_the_engine_are_floats(self):
        b, _ = court(yard={"kit": "yard", "bays": [13.0, 12.0]}, hall={"bays": [5.0, 3.0], "from": 24.0, "veranda": 1.0})
        self.assertEqual(b["c_hall"]["attrs"]["colonnade"], [{"sides": [0], "depth": 3.0}])


class CourtStep(unittest.TestCase):
    """court.py's step, through a fake of the SDK."""

    def run_step(self, **attrs):
        a = {k: v for k, v in {**COURT, **attrs}.items() if v is not None}
        s = FakeStep({"c": ([GATE], a)})
        s.id = "courts"
        K["generate"](s)
        return s

    def test_the_buildings_are_returned_with_their_attributes(self):
        s = self.run_step()
        self.assertEqual([x[:2] for x in s.said], [("info", "courts")])
        self.assertEqual(s.said[0][3], "1 courts, 4 buildings")
        f = s.out.features
        self.assertEqual({k: v["kind"] for k, v in f.items()},
                         {"c_yard": "building", "c_hall": "building", "c_w0_l": "building", "c_w0_r": "building"})
        self.assertEqual((f["c_yard"]["storeys"], f["c_hall"]["roof"], f["c_hall"]["region"][0]), (0, "hip", (24350, 34000)))

    def test_a_fault_is_said_of_its_court(self):
        s = self.run_step(hall={"bays": [4, 3], "from": 24})
        self.assertEqual([(x[0], x[1], x[2]) for x in s.said], [("error", "params", "c")])
        self.assertTrue(s.said[0][3].startswith("c: the hall is 4 bays across"))
        self.assertEqual(s.out.features, {})

    def test_eaves_that_meet_are_said_in_metres_with_what_to_move(self):
        s = self.run_step(wings=[{"bays": [3, 2], "from": 13, "off": 9}])
        w = [x for x in s.said if x[1] == "court_meets"]
        self.assertEqual(len(w), 2)
        self.assertEqual(w[0][3], "`c_w0_l`, with its eaves, stands 1 m into `c_hall` and its eaves")
        self.assertIn("move it 1 m or more", w[0][4])
        self.assertEqual(w[0][5], {"building": "c_w0_l", "other": "c_hall", "by": 1.0, "eaves": True, "returned": True})

    def test_a_hall_past_the_yards_wall_is_said_and_not_returned(self):
        s = self.run_step(wings=[{"bays": [3, 2], "from": 9, "off": 13.5}])
        w = [x for x in s.said if x[1] == "court_meets"]
        self.assertEqual(w[0][3], "`c_w0_l` passes the inner face of the yard's wall by 0.3 m: it is not returned")
        self.assertEqual(w[0][5], {"building": "c_w0_l", "other": "c_yard", "by": 0.3, "eaves": False, "returned": False})
        self.assertEqual(sorted(s.out.features), ["c_hall", "c_yard"])


if __name__ == "__main__":
    unittest.main()
