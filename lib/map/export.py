"""The player map as data (build.md 9.4): an export step that reads the finished build through the
SDK and writes the map's layers under ``export/<step id>/``. Style and discovery state are the
game's; this writes only what the world holds.

An export step (`when = "export"`, protocol.md 1.1), run as a plain command (`python -m viasdk
lib:map/export.py`): it is handed the finished build (fields, features with every attribute,
instances, the pack) and reads nothing else. What a kind means on the map is the world's to say,
in `params` (below): this library knows no kind but its own `poi`.

What it writes (README, "The map's files"; the rules are build.md 9.4's, marked ours there):

- ``map.json``: the index. `format` `via-map-0`, `output` `player_map`, the world-to-map
  `transform`, the ground `classes`, the `layers` and their files, the `floors`, the `tiles` with
  their fingerprints, and `discovery` (the grid a game keys what the player has seen on).
- ``ground.npy``: one class id per map cell, `|u1`, shape `(rows, columns)`, row 0 the south row
  (build.md 3.2's rule, so one reader reads both).
- ``vectors.json``: `ways`, `water`, `footprints`, `regions` of the ground plan.
- ``labels.json``: POIs and labels, each with a key the game localises and a floor (`null`: the
  ground plan).
- ``floors/<n>.json``: one plan per floor: its surfaces, rooms, wall pieces cut at `cut` metres
  over the floor, doors, and links (stairs) to other floors.

`params` (each kind named by the world; every one optional):

| Param | Default | Holds |
|---|---|---|
| `cell` | 1.0 | Map cell side, metres: the ground raster's resolution. |
| `tile` | 32 | Tile side in cells, for fingerprints. |
| `reveal` | 4.0 | Discovery cell side, metres (a whole number of cells). |
| `ways` | {} | `{ <path kind>: <class> }`: drawn as ways, `width` attribute (metres; `way_width` without). |
| `under` | [] | Path kinds of `ways` that run under the ground (a cave): in `vectors.json`, not in the raster. |
| `water` | {} | `{ <path kind>: <class> }`: a water course, drawn by its `width`. |
| `water_field` | "tag:water_level" | The water surface field; water where it stands `depth_min` over the ground. |
| `ground_field` | "role:surface" | The ground's height field. |
| `material` | "role:material" | The weights field whose largest weight at a cell is its ground class. |
| `regions` | {} | `{ <region kind>: <label attribute or "">}`: named regions. |
| `floors` | {} | `{ <region kind>: <level attribute> }`: kind surfaces that are floors. |
| `rooms` | {} | `{ <region kind>: <attribute naming its floor feature> }`. |
| `doors` | {} | `{ <path kind>: <height attribute> }`. |
| `links` | {} | `{ <path kind>: [<from height attribute>, <to height attribute>] }`: stairs, ramps. |
| `walls` | [] | Taxon paths of wall pieces drawn on floor plans. |
| `labels` | {} | `{ <kind>: <text attribute or "">}`: a label at each such feature ("" : its name). |
| `pois` | {"poi": "label"} | `{ <kind>: <text attribute> }`: points of interest. |
| `floor_gap` | 1.6 | Two floor levels further apart than this are two floors (metres). |
| `cut` | 1.2 | A floor plan is cut this far over the floor (metres). |
| `depth_min` | 0.05 | Water shallower than this is not water on the map (metres). |
| `way_width` | 2.0 | A way's width when its kind gives none (metres). |

Library code imports only viasdk and the standard library (R19b).
"""

import hashlib
import json
import math
import os

FORMAT = "via-map-0"
OUTPUT = "player_map"

DEFAULTS = {
    "cell": 1.0, "tile": 32, "reveal": 4.0, "ways": {}, "under": [], "water": {},
    "water_field": "tag:water_level", "ground_field": "role:surface", "material": "role:material",
    "regions": {}, "floors": {}, "rooms": {}, "doors": {}, "links": {}, "walls": [], "labels": {},
    "pois": {"poi": "label"}, "floor_gap": 1.6, "cut": 1.2, "depth_min": 0.05, "way_width": 2.0,
}

EXPLAIN = {
    "params": "A param of the map export is outside what it takes: `cell`, `reveal` and the lengths "
    "above 0, `reveal` a whole number of cells, `tile` a whole number of cells, kinds as tables.",
    "written": "What the map export wrote: cells, classes, ways, footprints, floors and tiles.",
}


class ParamError(ValueError):
    pass


def params_of(raw):
    p = dict(DEFAULTS)
    for k, v in (raw or {}).items():
        if k not in DEFAULTS:
            raise ParamError(f"`{k}` is not a param of the map export")
        p[k] = v
    for k in ("cell", "reveal", "floor_gap", "cut", "way_width"):
        if not isinstance(p[k], (int, float)) or p[k] <= 0:
            raise ParamError(f"`{k}` must be a number above 0")
    if not isinstance(p["tile"], int) or p["tile"] < 1:
        raise ParamError("`tile` must be a whole number of cells, 1 or more")
    ratio = p["reveal"] / p["cell"]
    if abs(ratio - round(ratio)) > 1e-9 or round(ratio) < 1:
        raise ParamError("`reveal` must be a whole number of cells")
    for k in ("ways", "water", "regions", "floors", "rooms", "doors", "links", "labels", "pois"):
        if not isinstance(p[k], dict):
            raise ParamError(f"`{k}` must be a table of kinds")
    return p


# --- numbers and JSON -------------------------------------------------------------------------


def m(mm):
    """Integer millimetres as exact metres (a lattice value reads back to the same millimetres)."""
    return round(int(mm) / 1000.0, 3)


def canonical(value):
    """Canonical JSON text (documents.md 8): sorted keys, no spaces, one LF at the end."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"


def fingerprint(data):
    """A tile's or a file's fingerprint: SHA-256 (FIPS 180-4) of its bytes, 64 hex digits. Not the
    build's BLAKE3 (build.md 3.3): library code has the standard library only. Ours."""
    return hashlib.sha256(data).hexdigest()


def npy_u1(rows):
    """``rows`` (lists of 0..255) as a NumPy 1.0 file, `|u1`, header padded to 64 bytes."""
    ny, nx = len(rows), len(rows[0]) if rows else 0
    head = "{'descr': '|u1', 'fortran_order': False, 'shape': (%d, %d), }" % (ny, nx)
    total = 10 + len(head) + 1
    pad = (64 - total % 64) % 64
    head = head + " " * pad + "\n"
    out = bytearray(b"\x93NUMPY\x01\x00")
    out += len(head).to_bytes(2, "little")
    out += head.encode("latin-1")
    for r in rows:
        out += bytes(r)
    return bytes(out)


# --- geometry ---------------------------------------------------------------------------------


def in_ring(ring, x, y):
    inside = False
    n = len(ring)
    for i in range(n):
        x1, y1 = ring[i]
        x2, y2 = ring[(i + 1) % n]
        if (y1 > y) != (y2 > y):
            xc = x1 + (y - y1) * (x2 - x1) / (y2 - y1)
            if x < xc:
                inside = not inside
    return inside


def in_pieces(pieces, x, y):
    for piece in pieces:
        if piece and in_ring(piece[0], x, y) and not any(in_ring(h, x, y) for h in piece[1:]):
            return True
    return False


def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    L = dx * dx + dy * dy
    t = 0.0 if L == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / L))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)


def line_dist(points, x, y):
    if len(points) == 1:
        return math.hypot(x - points[0][0], y - points[0][1])
    return min(seg_dist(x, y, *points[i], *points[i + 1]) for i in range(len(points) - 1))


def box_of(coords, grow=0):
    xs = [p[0] for p in coords]
    ys = [p[1] for p in coords]
    return (min(xs) - grow, min(ys) - grow, max(xs) + grow, max(ys) + grow)


def boxes_meet(a, b):
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]


def clip(a, b, box):
    """The part of segment a-b inside ``box`` (Liang and Barsky, 1984), or None."""
    t0, t1 = 0.0, 1.0
    dx, dy = b[0] - a[0], b[1] - a[1]
    for p, q in ((-dx, a[0] - box[0]), (dx, box[2] - a[0]), (-dy, a[1] - box[1]), (dy, box[3] - a[1])):
        if p == 0:
            if q < 0:
                return None
            continue
        r = q / p
        if p < 0:
            t0 = max(t0, r)
        else:
            t1 = min(t1, r)
        if t0 > t1:
            return None
    return [[round(a[0] + t0 * dx, 3), round(a[1] + t0 * dy, 3)],
            [round(a[0] + t1 * dx, 3), round(a[1] + t1 * dy, 3)]]


def in_tile(e, tb):
    """What of item ``e`` a tile's fingerprint covers: a path's segments clipped to the tile grown
    by half its width (so a moved end changes only the tiles it is drawn in); any other item
    whole. None when a path does not reach the tile."""
    pts = e.get("points")
    if not pts or len(pts) < 2:
        return e
    g = e.get("width", 0) / 2
    box = (tb[0] - g, tb[1] - g, tb[2] + g, tb[3] + g)
    segs = [c for c in (clip(pts[i], pts[i + 1], box) for i in range(len(pts) - 1)) if c is not None]
    if not segs:
        return None
    part = {k: v for k, v in e.items() if k != "points"}
    part["segments"] = segs
    return part


def label_point(pieces):
    """Where a region's label stands: the area centroid of its largest outer ring if that lies
    inside the region, else the middle of its longest inside chord on the centroid's row. Ours."""
    best, area_best = None, -1.0
    for piece in pieces:
        r = piece[0]
        a = 0.0
        cx = cy = 0.0
        for i in range(len(r)):
            x1, y1 = r[i]
            x2, y2 = r[(i + 1) % len(r)]
            c = x1 * y2 - x2 * y1
            a += c
            cx += (x1 + x2) * c
            cy += (y1 + y2) * c
        if abs(a) > area_best:
            area_best = abs(a)
            best = (r, a, cx, cy)
    r, a, cx, cy = best
    if a == 0:
        return [sum(p[0] for p in r) / len(r), sum(p[1] for p in r) / len(r)]
    cx, cy = cx / (3 * a), cy / (3 * a)
    if in_pieces(pieces, cx, cy):
        return [cx, cy]
    xs = []
    for i in range(len(r)):
        x1, y1 = r[i]
        x2, y2 = r[(i + 1) % len(r)]
        if (y1 > cy) != (y2 > cy):
            xs.append(x1 + (cy - y1) * (x2 - x1) / (y2 - y1))
    xs.sort()
    spans = [(xs[i + 1] - xs[i], (xs[i] + xs[i + 1]) / 2) for i in range(0, len(xs) - 1, 2)]
    return [max(spans)[1], cy] if spans else [r[0][0], r[0][1]]


def piece_box(asset_bounds, x, y, face, scale):
    """A piece's footprint in the world, metres: its bounds' plan box scaled and turned by its
    facing (build.md 8: R(b) takes the asset's +y to the bearing b, clockwise from north)."""
    x0, y0, _, x1, y1, _ = asset_bounds
    b = math.radians(face / 1000.0)
    c, s = math.cos(b), math.sin(b)
    ring = []
    for ax, ay in ((x0, y0), (x1, y0), (x1, y1), (x0, y1)):
        ax, ay = ax * scale, ay * scale
        ring.append([round(x + c * ax + s * ay, 3), round(y - s * ax + c * ay, 3)])
    return ring


# --- the world as handed ----------------------------------------------------------------------


def _list(v):
    return v.tolist() if hasattr(v, "tolist") else list(v)


def _attr(col, f):
    """One attribute value of feature row ``f``: metres for a lattice length (i32 millimetres,
    protocol.md 4.3), a list or table from its JSON text."""
    v = col[f]
    if hasattr(v, "item"):
        dt = getattr(col, "dtype", None)
        v = v.item()
        if dt is not None and str(dt) in ("int32", "<i4"):
            return v / 1000.0
    if isinstance(v, str) and v[:1] in ("[", "{"):
        try:
            return json.loads(v)
        except ValueError:
            return v
    return v


def _rings(fl, f):
    """Feature ``f``'s pieces, each a list of rings of ``[x, y]`` millimetres (protocol.md 4.3)."""
    pieces, rings, starts = fl["pieces"], fl["rings"], fl["ring_starts"]
    coords = _list(fl["coords"])
    out = []
    for q in range(int(pieces[f]), int(pieces[f + 1])):
        out.append([coords[int(starts[r]):int(starts[r + 1])]
                    for r in range(int(rings[q]), int(rings[q + 1]))])
    return out


def grid_of(g):
    """The request's grid (protocol.md 2.1, millimetres) in metres."""
    return {"extent": [m(v) for v in g["extent_mm"]], "resolution": m(g["resolution_mm"]),
            "nx": int(g["nx"]), "ny": int(g["ny"])}


def read_world(step, p):
    """Everything the map reads, through the SDK: features (footprints included), instances,
    the pack's bounds and taxons, and the ground, water and material fields."""
    fl = step.features("*")
    base = {"coords", "geometry", "pieces", "rings", "ring_starts", "name", "kind", "authority",
            "layer"}
    names = _list(fl.get("name", []))
    kinds = _list(fl.get("kind", []))
    geom = _list(fl.get("geometry", []))
    attr_cols = [k for k in fl if k not in base and not k.endswith("?")]
    features = []
    for f, name in enumerate(names):
        attrs = {}
        for a in attr_cols:
            has = fl.get(a + "?")
            if has is not None and not int(has[f]):
                continue
            attrs[a] = _attr(fl[a], f)
        rings = [[[(m(c[0]), m(c[1])) for c in r] for r in piece] for piece in _rings(fl, f)]
        g = int(geom[f])
        feat = {"name": name, "kind": kinds[f] or None, "attrs": attrs,
                "shape": ("point", "path", "region")[g]}
        if g == 2:
            feat["pieces"] = [[[list(v) for v in r] for r in piece] for piece in rings]
        else:
            feat["points"] = [list(v) for v in rings[0][0]]
        features.append(feat)

    inst = step.instances("*")
    instances = []
    if inst:
        n = len(_list(inst.get("name", [])))
        cols = {k: _list(inst[k]) for k in ("name", "asset", "x", "y", "z", "face", "scale") if k in inst}
        for i in range(n):
            instances.append({"name": cols["name"][i], "asset": cols["asset"][i],
                              "x": m(cols["x"][i]), "y": m(cols["y"][i]), "z": float(cols["z"][i]),
                              "face": int(cols["face"][i]), "scale": float(cols["scale"][i])})
    pack = {}
    try:
        pk = step.pack("*", columns=["bounds"])
        ids = _list(pk.get("id", []))
        bounds = _list(pk.get("bounds", []))
        taxa = _list(pk.get("taxon", [""] * len(ids)))
        for i, a in enumerate(ids):
            pack[a] = {"bounds": [v / 1000.0 for v in bounds[i]], "taxon": taxa[i] or ""}
    except Exception:  # noqa: BLE001 - a build with no pack: no wall pieces to draw
        pack = {}

    fields = {}
    for key in ("ground_field", "water_field", "material"):
        try:
            (a,) = step.fields([p[key]])
            fields[key] = a
        except Exception:  # noqa: BLE001 - a world without that field draws without it
            fields[key] = None
    return {"features": features, "instances": instances, "pack": pack, "fields": fields,
            "grid": grid_of(step.grid)}


# --- the map ----------------------------------------------------------------------------------


def transform(extent, cell):
    """The world-to-map transform: cell (c, r) has its centre at world (x0 + (c + 0.5) * cell,
    y0 + (r + 0.5) * cell); row 0 is the south row. `affine` takes world metres to cell units."""
    x0, y0, x1, y1 = extent
    nx = int(math.ceil((x1 - x0) / cell - 1e-9))
    ny = int(math.ceil((y1 - y0) / cell - 1e-9))
    return {"extent": [x0, y0, x1, y1], "cell": cell, "size": [nx, ny], "rows": "south first",
            "affine": [[1 / cell, 0, 0.0 - x0 / cell], [0, 1 / cell, 0.0 - y0 / cell]]}


def _sampler(arr, grid):
    """Nearest post of a field array at world (x, y) metres."""
    if arr is None:
        return None
    x0, y0 = grid["extent"][0], grid["extent"][1]
    res = grid["resolution"]
    i0, j0 = (arr.window[0], arr.window[1]) if hasattr(arr, "window") else (0, 0)

    def at(x, y, plane=None):
        i = int(round((x - x0) / res)) - i0
        j = int(round((y - y0) / res)) - j0
        if plane is None:
            return float(arr[j, i])
        return int(arr[plane, j, i])

    return at


def floors_of(features, p):
    """The floors: every feature of a `floors` kind with its level, grouped so that levels closer
    than `floor_gap` are one floor. Floor n's range is [lowest level - 0.5, the next floor's
    lowest - 0.5): a player whose feet are at z is on the highest floor whose range starts at or
    below z. Ours (the study found no documented height range)."""
    surf = []
    for f in features:
        attr = p["floors"].get(f["kind"]) if f["kind"] else None
        if attr is not None and f["shape"] == "region" and attr in f["attrs"]:
            surf.append((float(f["attrs"][attr]), f["name"], f))
    surf.sort(key=lambda t: (t[0], t[1]))
    groups = []
    for level, name, f in surf:
        if groups and level - groups[-1][-1][0] <= p["floor_gap"]:
            groups[-1].append((level, name, f))
        else:
            groups.append([(level, name, f)])
    floors = []
    for n, g in enumerate(groups):
        lo = round(g[0][0] - 0.5, 3)
        floors.append({"index": n, "levels": [round(t[0], 3) for t in g], "z": [lo, None],
                       "surfaces": [t[2] for t in g]})
    for n in range(len(floors) - 1):
        floors[n]["z"][1] = floors[n + 1]["z"][0]
    return floors


def floor_at(floors, z):
    best = None
    for fl in floors:
        if fl["z"][0] <= z + 1e-9:
            best = fl["index"]
    return best


def make_map(world, p):
    """The map's files, as ``{relative path: bytes}``, and a count of what was written."""
    grid = world["grid"]
    feats = world["features"]
    tr = transform(grid["extent"], p["cell"])
    nx, ny = tr["size"]
    cell = p["cell"]
    x0, y0 = grid["extent"][0], grid["extent"][1]

    # Vector layers of the ground plan.
    ways, water, regions, labels = [], [], [], []
    for f in feats:
        k = f["kind"]
        if k in p["ways"] and f["shape"] == "path":
            w = f["attrs"].get("width")
            ways.append({"name": f["name"], "kind": k, "class": p["ways"][k],
                         "width": float(w) if isinstance(w, (int, float)) else p["way_width"],
                         "under": k in p["under"], "points": [v[:2] for v in f["points"]]})
        if k in p["water"] and f["shape"] == "path":
            w = f["attrs"].get("width")
            water.append({"name": f["name"], "kind": k, "class": p["water"][k],
                          "width": float(w) if isinstance(w, (int, float)) else p["way_width"],
                          "points": [v[:2] for v in f["points"]]})
        if k in p["regions"] and f["shape"] == "region":
            a = p["regions"][k]
            regions.append({"name": f["name"], "kind": k, "pieces": f["pieces"],
                            "label": str(f["attrs"].get(a, f["name"])) if a else f["name"]})
        for table, cls in ((p["labels"], "label"), (p["pois"], "poi")):
            if k in table:
                a = table[k]
                at = (label_point(f["pieces"]) if f["shape"] == "region" else
                      f["points"][len(f["points"]) // 2] if f["shape"] == "path" else f["points"][0])
                entry = {"key": f["name"], "class": cls, "kind": k, "floor": None,
                         "text": str(f["attrs"].get(a, f["name"])) if a else f["name"],
                         "at": [round(at[0], 3), round(at[1], 3)]}
                if cls == "poi":
                    entry["icon"] = f["attrs"].get("icon", "")
                labels.append(entry)
    for r in regions:
        at = label_point(r["pieces"])
        labels.append({"key": r["name"], "class": "region", "kind": r["kind"], "floor": None,
                       "text": r["label"], "at": [round(at[0], 3), round(at[1], 3)]})

    floors = floors_of(feats, p)
    footprints = []
    for f in feats:
        if f["shape"] == "region" and f["name"].endswith(".ground"):
            footprints.append({"name": f["name"], "kind": f["kind"], "from": "footprint",
                               "pieces": f["pieces"]})
    if floors:
        for f in floors[0]["surfaces"]:
            footprints.append({"name": f["name"], "kind": f["kind"], "from": "floor",
                               "pieces": f["pieces"]})

    # The ground class raster: classes in a fixed order, a later one drawn over an earlier.
    sample_h = _sampler(world["fields"].get("ground_field"), grid)
    sample_w = _sampler(world["fields"].get("water_field"), grid)
    mat = world["fields"].get("material")
    sample_m = _sampler(mat, grid)
    materials = list(getattr(mat, "materials", None) or []) if mat is not None else []
    classes = [{"id": 0, "name": "none", "from": "no material"}]
    for name in sorted(materials):
        classes.append({"id": len(classes), "name": "ground:" + name, "from": "material"})
    water_id = len(classes)
    classes.append({"id": water_id, "name": "water", "from": "water"})
    water_ids = {}
    for c in sorted({w["class"] for w in water}):
        water_ids[c] = len(classes)
        classes.append({"id": water_ids[c], "name": "water:" + c, "from": "water course"})
    way_ids = {}
    for c in sorted({w["class"] for w in ways if not w["under"]}):
        way_ids[c] = len(classes)
        classes.append({"id": way_ids[c], "name": "way:" + c, "from": "way"})
    foot_id = len(classes)
    classes.append({"id": foot_id, "name": "footprint", "from": "footprint"})
    by_mat = {name: next(c["id"] for c in classes if c["name"] == "ground:" + name) for name in materials}

    raster = [[0] * nx for _ in range(ny)]
    for r in range(ny):
        y = y0 + (r + 0.5) * cell
        row = raster[r]
        for c in range(nx):
            x = x0 + (c + 0.5) * cell
            v = 0
            if sample_m is not None and materials:
                best, bw = None, 0
                for pl, name in enumerate(materials):
                    w = sample_m(x, y, pl)
                    if w > bw or (w == bw and best is not None and name < best):
                        best, bw = name, w
                if best is not None and bw > 0:
                    v = by_mat[best]
            if sample_w is not None and sample_h is not None:
                if sample_w(x, y) - sample_h(x, y) >= p["depth_min"]:
                    v = water_id
            row[c] = v

    def paint(box, test, value):
        c0 = max(0, int(math.floor((box[0] - x0) / cell)))
        c1 = min(nx - 1, int(math.floor((box[2] - x0) / cell)))
        r0 = max(0, int(math.floor((box[1] - y0) / cell)))
        r1 = min(ny - 1, int(math.floor((box[3] - y0) / cell)))
        for r in range(r0, r1 + 1):
            y = y0 + (r + 0.5) * cell
            for c in range(c0, c1 + 1):
                if test(x0 + (c + 0.5) * cell, y):
                    raster[r][c] = value

    for w in water:
        half = w["width"] / 2
        paint(box_of(w["points"], half), lambda x, y, w=w, h=half: line_dist(w["points"], x, y) <= h,
              water_ids[w["class"]])
    for w in ways:
        if w["under"]:
            continue
        half = w["width"] / 2
        paint(box_of(w["points"], half), lambda x, y, w=w, h=half: line_dist(w["points"], x, y) <= h,
              way_ids[w["class"]])
    for fp in footprints:
        outer = [v for piece in fp["pieces"] for v in piece[0]]
        paint(box_of(outer), lambda x, y, fp=fp: in_pieces(fp["pieces"], x, y), foot_id)

    # Floor plans.
    floor_docs = []
    names_on = {}
    for fl in floors:
        for s in fl["surfaces"]:
            names_on[s["name"]] = fl["index"]
    for fl in floors:
        n = fl["index"]
        lo = fl["levels"][0]
        surfaces = [{"name": s["name"], "kind": s["kind"],
                     "level": round(float(s["attrs"][p["floors"][s["kind"]]]), 3), "pieces": s["pieces"]}
                    for s in fl["surfaces"]]
        rooms = []
        for f in feats:
            a = p["rooms"].get(f["kind"]) if f["kind"] else None
            if a is None or f["shape"] != "region":
                continue
            on = f["attrs"].get(a)
            on = on.get("name") if isinstance(on, dict) else on
            if names_on.get(on) == n:
                at = label_point(f["pieces"])
                rooms.append({"name": f["name"], "kind": f["kind"], "on": on, "pieces": f["pieces"]})
                labels.append({"key": f["name"], "class": "room", "kind": f["kind"], "floor": n,
                               "text": f["name"], "at": [round(at[0], 3), round(at[1], 3)]})
        boxes = [box_of([v for piece in s["pieces"] for v in piece[0]], 1.0) for s in fl["surfaces"]]
        walls = []
        for inst in world["instances"]:
            a = world["pack"].get(inst["asset"])
            if a is None or not any(a["taxon"] == t or a["taxon"].startswith(t + "/") for t in p["walls"]):
                continue
            zb = a["bounds"]
            z_lo = inst["z"] + zb[2] * inst["scale"]
            z_hi = inst["z"] + zb[5] * inst["scale"]
            if not any(z_lo <= lev + p["cut"] <= z_hi for lev in fl["levels"]):
                continue
            ring = piece_box(zb, inst["x"], inst["y"], inst["face"], inst["scale"])
            if not any(boxes_meet(box_of(ring), b) for b in boxes):
                continue
            walls.append({"instance": inst["name"], "asset": inst["asset"], "outline": ring})
        doors, links = [], []
        for f in feats:
            k = f["kind"]
            if k in p["doors"] and f["shape"] == "path":
                z = f["attrs"].get(p["doors"][k])
                if isinstance(z, (int, float)) and floor_at(floors, float(z)) == n:
                    doors.append({"name": f["name"], "kind": k, "z": round(float(z), 3),
                                  "points": [v[:2] for v in f["points"]]})
            if k in p["links"] and f["shape"] == "path":
                za, zb2 = (f["attrs"].get(a) for a in p["links"][k])
                if not isinstance(za, (int, float)) or not isinstance(zb2, (int, float)):
                    continue
                fa, fb = floor_at(floors, float(za)), floor_at(floors, float(zb2))
                if n in (fa, fb):
                    links.append({"name": f["name"], "kind": k, "from": fa, "to": fb,
                                  "z": [round(float(za), 3), round(float(zb2), 3)],
                                  "points": [v[:2] for v in f["points"]]})
        for lst, key in ((walls, "instance"), (rooms, "name"), (doors, "name"), (links, "name"),
                         (surfaces, "name")):
            lst.sort(key=lambda e: e[key])
        floor_docs.append({"index": n, "levels": fl["levels"], "z": fl["z"], "cut": p["cut"],
                           "surfaces": surfaces, "rooms": rooms, "walls": walls, "doors": doors,
                           "links": links})

    for lst in (ways, water, regions, footprints):
        lst.sort(key=lambda e: e["name"])
    labels.sort(key=lambda e: (e["floor"] is not None, e["floor"] or 0, e["class"], e["key"]))
    vectors = {"footprints": footprints, "regions": regions, "water": water, "ways": ways}

    files = {}
    files["ground.npy"] = npy_u1(raster)
    files["vectors.json"] = canonical(vectors).encode("utf-8")
    files["labels.json"] = canonical({"labels": labels}).encode("utf-8")
    for fd in floor_docs:
        files[f"floors/{fd['index']}.json"] = canonical(fd).encode("utf-8")

    # Tile fingerprints: one per tile per plan, over the tile's cells and every item that reaches it.
    T = p["tile"]
    tx_n, ty_n = int(math.ceil(nx / T)), int(math.ceil(ny / T))

    def items_box(e):
        coords = []
        for key in ("points", "outline"):
            if key in e:
                coords += e[key]
        for piece in e.get("pieces", []):
            coords += piece[0]
        if "at" in e:
            coords.append(e["at"])
        if not coords:
            return None
        grow = e.get("width", 0) / 2
        return box_of(coords, grow)

    ground_items = ([("ways", e) for e in ways] + [("water", e) for e in water] +
                    [("footprints", e) for e in footprints] + [("regions", e) for e in regions] +
                    [("labels", e) for e in labels if e["floor"] is None])
    plans = [("ground", ground_items, raster)]
    for fd in floor_docs:
        its = ([(k, e) for k in ("surfaces", "rooms", "walls", "doors", "links") for e in fd[k]] +
               [("labels", e) for e in labels if e["floor"] == fd["index"]])
        plans.append((f"floor:{fd['index']}", its, None))
    tiles = []
    for plan, items, ras in plans:
        boxed = [(k, e, items_box(e)) for k, e in items]
        for ty in range(ty_n):
            for tx in range(tx_n):
                tb = (x0 + tx * T * cell, y0 + ty * T * cell,
                      x0 + min((tx + 1) * T, nx) * cell, y0 + min((ty + 1) * T, ny) * cell)
                h = hashlib.sha256()
                if ras is not None:
                    for r in range(ty * T, min((ty + 1) * T, ny)):
                        h.update(bytes(ras[r][tx * T:min((tx + 1) * T, nx)]))
                touched = []
                for k, e, b in boxed:
                    if b is None or not (b[0] < tb[2] and tb[0] <= b[2] and b[1] < tb[3] and tb[1] <= b[3]):
                        continue
                    part = in_tile(e, tb)
                    if part is not None:
                        touched.append([k, part])
                h.update(canonical(touched).encode("utf-8"))
                tiles.append({"plan": plan, "at": [tx, ty], "items": len(touched),
                              "fingerprint": h.hexdigest()})

    rv = int(round(p["reveal"] / cell))
    index = {
        "format": FORMAT,
        "output": OUTPUT,
        "transform": tr,
        "classes": classes,
        "layers": {"ground": "ground.npy", "vectors": "vectors.json", "labels": "labels.json"},
        "floors": [{"index": fd["index"], "levels": fd["levels"], "z": fd["z"],
                    "file": f"floors/{fd['index']}.json",
                    "fingerprint": fingerprint(files[f"floors/{fd['index']}.json"])}
                   for fd in floor_docs],
        "tiles": {"side": T, "count": [tx_n, ty_n], "list": tiles},
        "discovery": {"cell": p["reveal"], "cells": rv, "size": [int(math.ceil(nx / rv)),
                                                                   int(math.ceil(ny / rv))],
                      "rooms": "by name", "state": "the game's"},
        "files": {k: fingerprint(v) for k, v in sorted(files.items())},
    }
    files["map.json"] = canonical(index).encode("utf-8")
    counts = {"cells": nx * ny, "classes": len(classes), "ways": len(ways), "water": len(water),
              "footprints": len(footprints), "regions": len(regions), "labels": len(labels),
              "floors": len(floor_docs), "tiles": len(tiles),
              "walls": sum(len(fd["walls"]) for fd in floor_docs),
              "links": sum(len(fd["links"]) for fd in floor_docs)}
    return files, counts


def write(folder, files):
    for rel, data in sorted(files.items()):
        path = os.path.join(folder, *rel.split("/"))
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)


def export(step):
    try:
        p = params_of(step.params)
    except ParamError as e:
        step.error(f"step.{step.id}.params", f"step.{step.id}", str(e),
                   "see lib/map/export.py for the params it takes", explain=EXPLAIN["params"])
        return
    world = read_world(step, p)
    files, counts = make_map(world, p)
    write(step.out.folder, files)
    step.info(f"step.{step.id}.written", f"step.{step.id}",
              ", ".join(f"{k} {v}" for k, v in sorted(counts.items())),
              "read export/<step id>/map.json", explain=EXPLAIN["written"], data=counts)
