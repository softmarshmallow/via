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
- ``labels.json``: POIs and labels, each with a key the game localises, a floor (`null`: the
  ground plan) and the box its text is placed in (below), and the zoom they were placed at.
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
| `buildings` | [] | Taxon paths of assets whose instances are footprints on the ground plan (their bounds' plan box, turned and scaled as placed): a house with no `ground` feature. |
| `labels` | {} | `{ <kind>: <text attribute or "">}`: a label at each such feature ("": read from its name, `name_text`; a produced name gives none). |
| `pois` | {"poi": "label"} | `{ <kind>: <text attribute> }`: points of interest. |
| `floor_gap` | 1.6 | Two floor levels further apart than this are two floors (metres). |
| `cut` | 1.2 | A floor plan is cut this far over the floor (metres). |
| `depth_min` | 0.05 | Water shallower than this is not water on the map (metres). |
| `way_width` | 2.0 | A way's width when its kind gives none (metres). |
| `room_label` | "label" | A room's attribute giving its text; without it, `Room <floor>.<n>` (below). |
| `label_zoom` | 4.0 | Pixels per metre at which the ground plan's labels are kept apart (below). |
| `floor_label_zoom` | 16.0 | The same for each floor plan. |
| `glyph` | [7, 13] | A character's advance and a line's height, pixels, at those zooms: a bound on the reader's font (PIL's default font: at most 5.5 px a character on average, 12 px a line). |

Labels (ours; build.md 9.4): a text read from a name is the name's last part, `_` as a space
(`name_text`); a produced name (holding `#`) gives none, so no label shows a produced id. A room
with no `room_label` attribute and a produced name reads `Room <floor>.<n>`, its floor's such rooms
numbered from 1 in reading order of their label points (north to south in 1 m rows, then west to
east, then by name); the key stays the feature's name. Each plan's labels are then placed greedily (priority: region, poi, label, room;
then by key) at the first of eight positions round the anchor (a region's centred first) where
the text's box, `glyph` per character at the plan's zoom, meets no box already placed, covers no
other label's anchor and lies inside the extent; one that fits nowhere gets `box` null (shown only closer in, the game's call).
On the ground plan a box also covers no footprint where any of its positions can (M7): the first
position clear of footprints too, else the first clear of labels and anchors alone (a label over
a dense town's roofs rather than none).

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
    "buildings": [], "room_label": "label", "label_zoom": 4.0, "floor_label_zoom": 16.0, "glyph": [7, 13],
}

EXPLAIN = {
    "params": "A param of the map export is outside what it takes: `cell`, `reveal` and the lengths "
    "above 0, `reveal` a whole number of cells, `tile` a whole number of cells, kinds as tables.",
    "written": "What the map export wrote: cells, classes, ways, footprints, floors and tiles.",
    "asset_unknown": "An instance's asset is not in the build's pack (step.pack), so the map has no "
    "bounds to draw its footprint or walls from.",
}


class ParamError(ValueError):
    pass


def params_of(raw):
    p = dict(DEFAULTS)
    for k, v in (raw or {}).items():
        if k not in DEFAULTS:
            raise ParamError(f"`{k}` is not a param of the map export")
        p[k] = v
    for k in ("cell", "reveal", "floor_gap", "cut", "way_width", "label_zoom", "floor_label_zoom"):
        if not isinstance(p[k], (int, float)) or p[k] <= 0:
            raise ParamError(f"`{k}` must be a number above 0")
    if not isinstance(p["tile"], int) or p["tile"] < 1:
        raise ParamError("`tile` must be a whole number of cells, 1 or more")
    g = p["glyph"]
    if not (isinstance(g, list) and len(g) == 2 and all(isinstance(v, (int, float)) and v > 0 for v in g)):
        raise ParamError("`glyph` must be [advance, line height], pixels above 0")
    ratio = p["reveal"] / p["cell"]
    if abs(ratio - round(ratio)) > 1e-9 or round(ratio) < 1:
        raise ParamError("`reveal` must be a whole number of cells")
    if not isinstance(p["buildings"], list) or not isinstance(p["walls"], list):
        raise ParamError("`walls` and `buildings` must be lists of taxon paths")
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


def name_text(name):
    """A label's text read from a feature's name when the world gives no text attribute: its own
    last part (`coast.sea_cave` reads `Sea cave`), `_` as a space, the first letter capital. A
    produced name (it holds `#`: a step's key, often a digest) gives none: the feature has no
    label, and the step counts it `unnamed`. The key is the full name either way. Ours."""
    last = name.rsplit(".", 1)[-1]
    if "#" in last:
        return None
    text = last.replace("_", " ").strip()
    return text[:1].upper() + text[1:] if text else None


def under_taxa(taxon, paths):
    return any(taxon == t or taxon.startswith(t + "/") for t in paths)


def name_rooms(labels):
    """A room's text when the world gives none: its name's (`name_text`), and for a produced name
    `Room <floor>.<n>`, n from 1 over that floor's produced rooms in reading order of their label
    points (north to south in 1 m rows, west to east, then by key). Ours."""
    by_floor = {}
    for e in labels:
        if e["class"] == "room" and e["text"] is None:
            e["text"] = name_text(e["key"])
            if e["text"] is None:
                by_floor.setdefault(e["floor"], []).append(e)
    for n, rooms in by_floor.items():
        rooms.sort(key=lambda e: (-math.floor(e["at"][1]), e["at"][0], e["key"]))
        for i, e in enumerate(rooms):
            e["text"] = f"Room {n}.{i + 1}"


PRIORITY = {"region": 0, "poi": 1, "label": 2, "room": 3}
# Where a point's text may stand, in order of preference (Imhof 1975: upper right first): the
# box's corner or side nearest the anchor, as (dx, dy) of the box's centre in box half-sizes.
POSITIONS = [(1, 1), (-1, 1), (1, -1), (-1, -1), (1, 0), (-1, 0), (0, 1), (0, -1)]


def box_meets_ring(b, ring):
    """Whether the open box b (x0, y0, x1, y1) and the polygon ring meet: a vertex of one inside
    the other, or an edge of the ring crossing the box."""
    if any(b[0] < x < b[2] and b[1] < y < b[3] for x, y in ring):
        return True
    if any(in_ring(ring, x, y) for x, y in ((b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3]))):
        return True
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        # Liang-Barsky: does the segment enter the box's interior?
        t0, t1 = 0.0, 1.0
        dx, dy = bx - ax, by - ay
        for pp, qq in ((-dx, ax - b[0]), (dx, b[2] - ax), (-dy, ay - b[1]), (dy, b[3] - ay)):
            if pp == 0:
                if qq <= 0:
                    t0, t1 = 1.0, 0.0
                    break
                continue
            r = qq / pp
            if pp < 0:
                t0 = max(t0, r)
            else:
                t1 = min(t1, r)
        if t0 < t1:
            return True
    return False


def place_labels(labels, zoom, glyph, extent, footprints=()):
    """Give each label a `box` (world metres, x0 y0 x1 y1) where its text stands clear of every
    other label's at ``zoom`` pixels per metre, greedily (module docstring), or null; clear of
    ``footprints`` (vectors.json's) where any position can be. Ours."""
    gap = 2.0 / zoom  # metres between the anchor and the text
    placed = []
    anchors = [(e["key"], e["at"][0], e["at"][1]) for e in labels]
    rings = [(box_of(pc[0]), [tuple(v[:2]) for v in pc[0]]) for f in footprints for pc in f["pieces"] if pc]
    for e in sorted(labels, key=lambda e: (PRIORITY.get(e["class"], 9), e["key"])):
        w = (len(e["text"]) * glyph[0] + 2) / zoom
        h = (glyph[1] + 2) / zoom
        cands = ([(0, 0)] if e["class"] == "region" else []) + POSITIONS
        e["box"] = None
        free = []  # the positions clear of labels, anchors and the extent's edge, in order
        for dx, dy in cands:
            cx = e["at"][0] + dx * (w / 2 + gap)
            cy = e["at"][1] + dy * (h / 2 + gap)
            b = (cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2)
            if b[0] < extent[0] or b[1] < extent[1] or b[2] > extent[2] or b[3] > extent[3]:
                continue
            if any(b[0] < q[2] and q[0] < b[2] and b[1] < q[3] and q[1] < b[3] for q in placed):
                continue
            if any(k != e["key"] and b[0] < x < b[2] and b[1] < y < b[3] for k, x, y in anchors):
                continue
            free.append(b)
        if not free:
            continue
        clear = [b for b in free if not any(b[0] < q[2] and q[0] < b[2] and b[1] < q[3] and q[1] < b[3]
                                            and box_meets_ring(b, ring) for q, ring in rings)]
        b = (clear or free)[0]
        placed.append(b)
        e["box"] = [round(v, 3) for v in b]


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
    # A build with no pack answers an empty table; any failure to read it is the engine's and
    # is raised, never drawn as a map with no footprints.
    pack = {}
    pk = step.pack("*", columns=["bounds"])
    ids = _list(pk.get("id", []))
    bounds = _list(pk.get("bounds", []))
    taxa = _list(pk.get("taxon", [""] * len(ids)))
    for i, a in enumerate(ids):
        pack[a] = {"bounds": [v / 1000.0 for v in bounds[i]], "taxon": taxa[i] or ""}
    unknown = sorted({i["asset"] for i in instances} - set(pack))
    if unknown:
        step.warning("asset_unknown", f"step.{step.id}",
                     f"{len(unknown)} instanced assets are not in the build's pack, so they draw no "
                     f"footprint: {', '.join(unknown[:8])}" + (" ..." if len(unknown) > 8 else ""),
                     "list the asset in a pack the world binds",
                     explain=EXPLAIN["asset_unknown"], data={"assets": unknown})

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


def _lists(arr):
    """A field array as nested lists, indexed [j][i] (or [plane][j][i]); None without one."""
    if arr is None:
        return None
    if hasattr(arr, "tolist"):
        return arr.tolist()
    i0, j0, i1, j1 = arr.window
    if getattr(arr, "materials", None):
        return [[[arr[pl, j, i] for i in range(i1 - i0)] for j in range(j1 - j0)]
                for pl in range(len(arr.materials))]
    return [[arr[j, i] for i in range(i1 - i0)] for j in range(j1 - j0)]


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
    unnamed = 0
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
                            "label": str(f["attrs"].get(a, f["name"])) if a else (name_text(f["name"]) or "")})
        for table, cls in ((p["labels"], "label"), (p["pois"], "poi")):
            if k in table:
                a = table[k]
                at = (label_point(f["pieces"]) if f["shape"] == "region" else
                      f["points"][len(f["points"]) // 2] if f["shape"] == "path" else f["points"][0])
                text = str(f["attrs"].get(a, f["name"])) if a else name_text(f["name"])
                if text is None:
                    unnamed += 1
                    continue
                entry = {"key": f["name"], "class": cls, "kind": k, "floor": None,
                         "text": text, "at": [round(at[0], 3), round(at[1], 3)]}
                if cls == "poi":
                    entry["icon"] = f["attrs"].get("icon", "")
                labels.append(entry)
    for r in regions:
        if not r["label"]:
            unnamed += 1
            continue
        at = label_point(r["pieces"])
        labels.append({"key": r["name"], "class": "region", "kind": r["kind"], "floor": None,
                       "text": r["label"], "at": [round(at[0], 3), round(at[1], 3)]})

    floors = floors_of(feats, p)
    footprints = []
    for f in feats:
        if f["shape"] == "region" and f["name"].endswith(".ground"):
            footprints.append({"name": f["name"], "kind": f["kind"], "from": "footprint",
                               "pieces": f["pieces"]})
    for inst in world["instances"]:
        a = world["pack"].get(inst["asset"])
        if a is not None and under_taxa(a["taxon"], p["buildings"]):
            ring = piece_box(a["bounds"], inst["x"], inst["y"], inst["face"], inst["scale"])
            footprints.append({"name": inst["name"], "kind": inst["asset"], "from": "instance",
                               "pieces": [[ring]]})
    if floors:
        for f in floors[0]["surfaces"]:
            footprints.append({"name": f["name"], "kind": f["kind"], "from": "floor",
                               "pieces": f["pieces"]})

    # The ground class raster: classes in a fixed order, a later one drawn over an earlier.
    mat = world["fields"].get("material")
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

    # Each cell reads its nearest post (build.md 9.4): the post indices per column and per row,
    # once; the fields as lists, once.
    res = grid["resolution"]

    def posts(n, origin, arr, axis):
        off = arr.window[axis] if arr is not None and hasattr(arr, "window") else 0
        return [int(round((origin + (k + 0.5) * cell - (grid["extent"][axis])) / res)) - off
                for k in range(n)]

    hd, wd, md = (_lists(world["fields"].get(k)) for k in ("ground_field", "water_field", "material"))
    raster = [[0] * nx for _ in range(ny)]
    use_m = md is not None and materials
    use_w = hd is not None and wd is not None
    ci_m = posts(nx, x0, mat, 0) if use_m else None
    rj_m = posts(ny, y0, mat, 1) if use_m else None
    ci_h = posts(nx, x0, world["fields"].get("ground_field"), 0) if use_w else None
    rj_h = posts(ny, y0, world["fields"].get("ground_field"), 1) if use_w else None
    ci_w = posts(nx, x0, world["fields"].get("water_field"), 0) if use_w else None
    rj_w = posts(ny, y0, world["fields"].get("water_field"), 1) if use_w else None
    order = sorted(range(len(materials)), key=lambda pl: materials[pl])  # ties to the lower name
    dmin = p["depth_min"]
    for r in range(ny):
        row = raster[r]
        if use_m:
            planes = [md[pl][rj_m[r]] for pl in order]
            ids = [by_mat[materials[pl]] for pl in order]
            for c in range(nx):
                i = ci_m[c]
                best, bw = 0, 0
                for k, pr in enumerate(planes):
                    w = pr[i]
                    if w > bw:
                        best, bw = ids[k], w
                row[c] = best
        if use_w:
            hr, wr = hd[rj_h[r]], wd[rj_w[r]]
            for c in range(nx):
                if wr[ci_w[c]] - hr[ci_h[c]] >= dmin:
                    row[c] = water_id

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

    def stroke(pts, half, value):
        """A line drawn by its width: every cell whose centre lies within ``half`` of a segment,
        segment by segment. ``half`` is at least half a cell's diagonal, so every cell the line
        passes through is drawn (a cell's centre is within that of each of its points): a way
        narrower than a cell is never broken. Ours."""
        half = max(half, cell * math.sqrt(2) / 2)
        if len(pts) == 1:
            paint(box_of(pts, half), lambda x, y: line_dist(pts, x, y) <= half, value)
        for i in range(len(pts) - 1):
            a, b = pts[i], pts[i + 1]
            paint(box_of([a, b], half), lambda x, y, a=a, b=b: seg_dist(x, y, *a, *b) <= half, value)

    for w in water:
        stroke(w["points"], w["width"] / 2, water_ids[w["class"]])
    for w in ways:
        if not w["under"]:
            stroke(w["points"], w["width"] / 2, way_ids[w["class"]])
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
                own = f["attrs"].get(p["room_label"])
                labels.append({"key": f["name"], "class": "room", "kind": f["kind"], "floor": n,
                               "text": str(own) if own not in (None, "") else None,
                               "at": [round(at[0], 3), round(at[1], 3)]})
        boxes = [box_of([v for piece in s["pieces"] for v in piece[0]], 1.0) for s in fl["surfaces"]]
        walls = []
        for inst in world["instances"]:
            a = world["pack"].get(inst["asset"])
            if a is None or not under_taxa(a["taxon"], p["walls"]):
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
    name_rooms(labels)
    for plan_floor, zoom in [(None, p["label_zoom"])] + [(fd["index"], p["floor_label_zoom"]) for fd in floor_docs]:
        place_labels([e for e in labels if e["floor"] == plan_floor], zoom, p["glyph"], grid["extent"],
                     footprints if plan_floor is None else ())
    labels.sort(key=lambda e: (e["floor"] is not None, e["floor"] or 0, e["class"], e["key"]))
    vectors = {"footprints": footprints, "regions": regions, "water": water, "ways": ways}

    files = {}
    files["ground.npy"] = npy_u1(raster)
    files["vectors.json"] = canonical(vectors).encode("utf-8")
    files["labels.json"] = canonical({"labels": labels, "placement": {
        "zoom": {"ground": p["label_zoom"], "floors": p["floor_label_zoom"]}, "glyph": p["glyph"],
        "rule": "greedy by class then key, eight positions, no two boxes meet (lib/map)"}}).encode("utf-8")
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
        if e.get("box"):
            b = e["box"]
            coords += [[b[0], b[1]], [b[2], b[3]]]
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
    side = T * cell
    for plan, items, ras in plans:
        # Each item into the tiles its box may meet (in item order, so a tile's list is the same
        # as testing every item), then the exact test per tile.
        buckets = {}
        for k, e in items:
            b = items_box(e)
            if b is None:
                continue
            for ty in range(max(0, int((b[1] - y0) // side) - 1), min(ty_n, int((b[3] - y0) // side) + 2)):
                for tx in range(max(0, int((b[0] - x0) // side) - 1), min(tx_n, int((b[2] - x0) // side) + 2)):
                    buckets.setdefault((tx, ty), []).append((k, e, b))
        for ty in range(ty_n):
            for tx in range(tx_n):
                tb = (x0 + tx * T * cell, y0 + ty * T * cell,
                      x0 + min((tx + 1) * T, nx) * cell, y0 + min((ty + 1) * T, ny) * cell)
                h = hashlib.sha256()
                if ras is not None:
                    for r in range(ty * T, min((ty + 1) * T, ny)):
                        h.update(bytes(ras[r][tx * T:min((tx + 1) * T, nx)]))
                touched = []
                for k, e, b in buckets.get((tx, ty), ()):
                    if not (b[0] < tb[2] and tb[0] <= b[2] and b[1] < tb[3] and tb[1] <= b[3]):
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
              "links": sum(len(fd["links"]) for fd in floor_docs), "unnamed": unnamed}
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
        step.error("params", f"step.{step.id}", str(e),
                   "see lib/map/export.py for the params it takes", explain=EXPLAIN["params"])
        return
    world = read_world(step, p)
    files, counts = make_map(world, p)
    write(step.out.folder, files)
    step.info("written", f"step.{step.id}",
              ", ".join(f"{k} {v}" for k, v in sorted(counts.items())),
              "read export/<step id>/map.json", explain=EXPLAIN["written"], data=counts)
