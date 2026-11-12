"""The world being compiled: its frame, its features' shapes, and its fields.

Owns `Grid` (the world rectangle and its sample grid), `Shape` (a feature's geometry and the local
frame it defines) and `World`: frames of other features, the expression environment, field
resolution, the effects kinds have on fields, the authority order those effects are applied in and
the conflicts between them, and the features derived from a field (floods).

Concepts, and nothing else:
  frame    the world rectangle, its sample grid, and the local frame every feature defines
  field    a function over the frame, resolved to a raster (scalar, or a set of material weights)
  feature  a point, path or region with a kind and free attributes
  kind     a named bundle of effects a feature has on fields (this is the seam model)
  instance an asset reference plus a transform
  asset    an opaque ID bound by a pack to a file and declared facts
  variant  an overlay document merged over the base
"""
from __future__ import annotations

import math

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from contourpy import contour_generator
from scipy.ndimage import binary_fill_holes, distance_transform_edt, label, map_coordinates, uniform_filter1d
from scipy.spatial import cKDTree
from shapely.geometry import LineString, Point, Polygon
from shapely.ops import substring

from .diagnostics import Diagnostics
from .geometry import PathGeom, _perlin, ellipse_polygon, smooth_path, smoothstep


class Unresolved(NameError):
    """A name the world cannot give a value to: a field or feature nobody declares, or a field that
    is part of a cycle. It is a `NameError` so callers that already catch one still do, and it is its
    own class so that an expression which merely fails to evaluate is never mistaken for it."""

    def __init__(self, code: str, message: str, chain: tuple = ()) -> None:
        super().__init__(message)
        self.code, self.chain = code, tuple(chain)


_MISSING = object()
_ATTR_FUNCS = {"tan": math.tan, "sin": math.sin, "cos": math.cos, "radians": math.radians, "degrees": math.degrees,
               "sqrt": math.sqrt, "min": min, "max": max, "abs": abs, "pi": math.pi}


def resolve_attributes(features: list[dict], kinds: dict, params: dict, diag: Diagnostics) -> None:
    """Work out every attribute written as a formula. A value that is a string starting with `=` is
    a formula over numbers: the feature's own attributes, the world's `[params]`, and another
    feature's attributes written `id.name`. A kind's defaults may be formulas too, so every feature
    of that kind gets them (a block's `top`, say). Each is worked out once, in whatever order the
    references need; a circle is an error that names its chain. Afterwards every such attribute is
    a plain number on the feature, and nothing downstream knows it was ever a formula."""
    by_id = {f["id"]: f for f in features if f.get("id")}
    done: dict = {}
    doing: list = []

    def is_formula(v) -> bool:
        return isinstance(v, str) and v.startswith("=")

    def raw(owner, key):
        if owner is None:
            return params.get(key, _MISSING)
        feat = by_id[owner]
        if key in feat:
            return feat[key]
        return kinds.get(feat.get("kind", ""), {}).get("defaults", {}).get(key, _MISSING)

    class Ref:
        def __init__(self, fid):
            self._fid = fid

        def __getattr__(self, key):
            got = value(self._fid, key)
            if got is _MISSING:
                raise NameError(f"'{self._fid}' has no attribute '{key}'")
            return got

    class Names(dict):
        def __init__(self, owner):
            super().__init__()
            self._owner = owner

        def __missing__(self, name):
            if name in _ATTR_FUNCS:
                return _ATTR_FUNCS[name]
            if self._owner is not None:
                own = value(self._owner, name)
                if own is not _MISSING:
                    return own
            par = value(None, name)
            if par is not _MISSING:
                return par
            if name in by_id:
                return Ref(name)
            raise NameError(f"'{name}' is not an attribute here, a parameter or a feature")

    def value(owner, key):
        r = raw(owner, key)
        if r is _MISSING or not is_formula(r):
            return r
        at = (owner, key)
        if at in done:
            return done[at]
        if at in doing:
            chain = " -> ".join(f"{o or 'params'}.{k}" for o, k in doing[doing.index(at):] + [at])
            raise RecursionError(chain)
        doing.append(at)
        try:
            done[at] = float(eval(r[1:], {"__builtins__": {}}, Names(owner)))
        finally:
            doing.pop()
        return done[at]

    def settle(owner, key, where: dict) -> None:
        subject = owner or "params"
        try:
            where[key] = value(owner, key)
        except RecursionError as exc:
            diag.error("attr.cycle", subject, f"`{key}` depends on itself: {exc}")
            where[key] = 0.0
        except Exception as exc:
            diag.error("attr.invalid", subject, f"cannot work out `{key}` {raw(owner, key)!r}: {exc}")
            where[key] = 0.0

    for key in [k for k, v in params.items() if is_formula(v)]:
        settle(None, key, params)
    for feat in features:
        fid = feat.get("id")
        if not fid:
            continue
        defaults = kinds.get(feat.get("kind", ""), {}).get("defaults", {})
        for key in [k for k, v in feat.items() if is_formula(v)] + [k for k, v in defaults.items() if is_formula(v) and k not in feat]:
            settle(fid, key, feat)


class Grid:
    def __init__(self, extent, res: float) -> None:
        self.x0, self.y0, self.x1, self.y1 = map(float, extent)
        self.res = float(res)
        self.nx = int(round((self.x1 - self.x0) / res)) + 1
        self.ny = int(round((self.y1 - self.y0) / res)) + 1
        xs = self.x0 + np.arange(self.nx, dtype=np.float32) * res
        ys = self.y0 + np.arange(self.ny, dtype=np.float32) * res
        self.X, self.Y = np.meshgrid(xs, ys)

    def window(self, bbox, margin: float):
        """Index slices covering a bbox grown by a margin, clipped to the grid."""
        i0 = max(0, int(math.floor((bbox[0] - margin - self.x0) / self.res)))
        j0 = max(0, int(math.floor((bbox[1] - margin - self.y0) / self.res)))
        i1 = min(self.nx, int(math.ceil((bbox[2] + margin - self.x0) / self.res)) + 1)
        j1 = min(self.ny, int(math.ceil((bbox[3] + margin - self.y0) / self.res)) + 1)
        return slice(j0, max(j0, j1)), slice(i0, max(i0, i1))

    def sample(self, arr: np.ndarray, xs, ys) -> np.ndarray:
        """Bilinear sample of a raster at world positions."""
        fx = np.clip((np.asarray(xs, dtype=np.float64) - self.x0) / self.res, 0, self.nx - 1.001)
        fy = np.clip((np.asarray(ys, dtype=np.float64) - self.y0) / self.res, 0, self.ny - 1.001)
        i, j = fx.astype(np.int64), fy.astype(np.int64)
        u, v = fx - i, fy - j
        return (arr[j, i] * (1 - u) * (1 - v) + arr[j, i + 1] * u * (1 - v)
                + arr[j + 1, i] * (1 - u) * v + arr[j + 1, i + 1] * u * v)

    def inside(self, x: float, y: float) -> bool:
        return self.x0 <= x <= self.x1 and self.y0 <= y <= self.y1


class Shape:
    """A feature's geometry and the local frame it defines over a window of the grid."""

    def __init__(self, world: "World", feat: dict) -> None:
        self.feat = feat
        self.grid = world.grid
        self.path: PathGeom | None = None
        self.outline: PathGeom | None = None
        self.mask = None
        self.polygon: list | None = None
        self.point = None
        if "path" in feat:
            self.type = "path"
            self.path = PathGeom(feat["path"], smooth=feat.get("smooth", True))
            self.bbox = self.path.bbox()
        elif "region" in feat or "ellipse" in feat:
            self.type = "region"
            self.polygon = feat.get("region") or ellipse_polygon(feat["ellipse"]["center"], feat["ellipse"]["size"])
            p = np.asarray(self.polygon, dtype=np.float64)
            area2 = float(np.sum(p[:, 0] * np.roll(p[:, 1], -1) - np.roll(p[:, 0], -1) * p[:, 1]))
            if area2 < 0:                      # keep outlines counter-clockwise: left is inside
                p = p[::-1]
                self.polygon = p.tolist()
            self.bbox = (p[:, 0].min(), p[:, 1].min(), p[:, 0].max(), p[:, 1].max())
            self.outline = PathGeom(p.tolist() + [p[0].tolist()], smooth=False)
            self.mask = feat.get("_mask")      # a derived region knows its cells exactly
        elif "at" in feat:
            self.type = "point"
            self.point = np.asarray(feat["at"], dtype=np.float64)
            self.bbox = (self.point[0], self.point[1], self.point[0], self.point[1])
        else:
            raise ValueError(f"feature '{feat.get('id')}' has no path, region, ellipse or at")
        self._cache: dict = {}

    def frame(self, reach: float) -> dict:
        """Over the window within `reach`: d (distance outside, 0 inside or on), sd (signed: negative inside
        a region, positive on the left of a path), t (fraction along a path), idx (nearest path sample)."""
        if reach in self._cache:
            return self._cache[reach]
        g = self.grid
        win = g.window(self.bbox, reach)
        X, Y = g.X[win], g.Y[win]
        out: dict = {"win": win}
        if self.type == "path":
            pts = np.stack([X.ravel(), Y.ravel()], axis=1)
            dist, idx = self.path.tree.query(pts)
            near = self.path.pts[idx]
            tan = self.path.tan[idx]
            cross = tan[:, 0] * (pts[:, 1] - near[:, 1]) - tan[:, 1] * (pts[:, 0] - near[:, 0])
            out["d"] = dist.reshape(X.shape).astype(np.float32)
            out["sd"] = (dist * np.sign(cross)).reshape(X.shape).astype(np.float32)
            out["t"] = (self.path.s[idx] / max(self.path.length, 1e-9)).reshape(X.shape).astype(np.float32)
            out["idx"] = idx.reshape(X.shape)
        elif self.type == "region":
            if self.mask is not None:          # derived: islands and all, at grid accuracy
                mask = self.mask[win]
                outside = distance_transform_edt(~mask) * g.res
                inside = distance_transform_edt(mask) * g.res
                out["sd"] = (outside - inside).astype(np.float32)
                out["d"] = outside.astype(np.float32)
                self._cache[reach] = out
                return out
            h, w = X.shape
            img = Image.new("L", (w, h), 0)
            j0, i0 = win[0].start, win[1].start
            px = [((x - g.x0) / g.res - i0, (y - g.y0) / g.res - j0) for x, y in self.polygon]
            ImageDraw.Draw(img).polygon(px, fill=1)
            mask = np.asarray(img, dtype=bool)
            # the raster gives the sign; the distance is measured to the true outline, not to the raster
            ring = np.asarray(self.polygon + [self.polygon[0]], dtype=np.float64)
            edge = smooth_path(ring.tolist(), spacing=0.1, smooth=False)
            dist = cKDTree(edge).query(np.stack([X.ravel(), Y.ravel()], axis=1))[0].reshape(X.shape)
            out["sd"] = np.where(mask, -dist, dist).astype(np.float32)
            out["d"] = np.where(mask, 0.0, dist).astype(np.float32)
        else:
            d = np.hypot(X - self.point[0], Y - self.point[1]).astype(np.float32)
            out["d"], out["sd"] = d, d
        self._cache[reach] = out
        return out


class World:
    def __init__(self, doc: dict, pack: dict, diag: Diagnostics, outer: dict | None = None, outer_paths: dict | None = None,
                 outer_faces: dict | None = None) -> None:
        self.doc, self.pack, self.diag = doc, pack, diag
        w = doc["world"]
        self.grid = Grid(w["extent"], w.get("resolution", 0.5))
        self.seed = int(w.get("seed", 0))
        self.params = dict(doc.get("params", {}))
        self.kinds: dict = doc.get("kind", {})
        self.features: list[dict] = [dict(f) for f in doc.get("feature", [])]
        resolve_attributes(self.features, self.kinds, self.params, diag)
        self.shapes: dict[str, Shape] = {}
        self.fields: dict[str, np.ndarray] = {}
        self.materials: list[str] = []          # order of the material field's weight planes
        self._resolving: list[str] = []
        self._cycles: set = set()               # cycles already reported, so each is said once
        for feat in self.features:               # a 3D path casts its plan as the feature's path
            if "path3" in feat and "path" not in feat:
                feat["path"] = [[p[0], p[1]] for p in feat["path3"]]
                feat.setdefault("smooth", False)
        self.derived_ids = {f.get("id") for f in self.features if "flood" in f or "where" in f}
        self._on_early: set = set()      # points placed `on` a feature that a path end asked for before their turn
        self._flooded: set = set()              # floods already attempted, whether or not they held water
        self.deferred: list[dict] = []
        self.placed: list[dict] = []            # every instance placed so far, so later ones can stand on earlier ones
        # A path may start or end at a mount: a named place on something else. Such a path cannot
        # have its shape until that place is known, so it waits while everything else is shaped.
        self.mounts: dict = dict(outer or {})   # "owner.name" -> (x, y); a master starts with the places its parts bring
        self.lock_dir = None                    # where locks keep their records; the pipeline says
        self.part_w: dict = {}                  # part id -> its blend weight over the whole grid, once it is joined
        self._part_d: dict = {}
        self.gaps: dict = {}                    # "<feature>.gap.<n>" -> (x, y, width): the openings left by `open_at`
        self.late_layers = {l["id"] for l in doc.get("layer", []) if l.get("after") == "parts"}
        self._named_paths(outer_paths or {})
        waiting = [f for f in self.features if "starts_at" in f or "ends_at" in f]
        for feat in self.features:
            if "on" not in feat and "flood" not in feat and "where" not in feat and not any(feat is w for w in waiting):
                self._add_shape(feat)
        while waiting:                          # one that ends on another waiting path takes a later turn
            ready = [f for f in waiting if all(self.mount(f[k], f.get("id", "?"), quiet=True) is not None
                                               for k in ("starts_at", "ends_at") if k in f)]
            if not ready:
                for f in waiting:
                    for k in ("starts_at", "ends_at"):
                        if k in f:
                            self.mount(f[k], f.get("id", "?"))      # say why
                break
            for f in ready:
                pts = [list(p) for p in f.get("path", [])]
                if "starts_at" in f:
                    pts = [list(self.mount(f["starts_at"], f["id"]))] + pts
                if "ends_at" in f:
                    pts = pts + [list(self.mount(f["ends_at"], f["id"]))]
                if "onward" in f:               # it leads nowhere: straight on from its start, the way that place faces
                    face = (outer_faces or {}).get(f.get("starts_at"))
                    if face is None or "ends_at" in f:
                        self.diag.error("frame.unknown", f["id"], "`onward` needs `starts_at` naming an exit of a part, and no `ends_at`",
                                        hint="an exit faces out of its part; a path end or a mount faces nowhere")
                    else:
                        last = pts[-1]
                        pts = pts + [[last[0] + face[0] * float(f["onward"]), last[1] + face[1] * float(f["onward"])]]
                f["path"] = pts
                self._add_shape(f)
                waiting = [w for w in waiting if w is not f]
        # things placed `on` a feature come after, since that feature may be one that waited
        for feat in self.features:
            if "on" in feat and feat.get("id") not in self._on_early:
                if feat["on"].get("feature") in self.derived_ids:
                    self.deferred.append(feat)      # its host does not exist until the terrain does
                else:
                    self._resolve_on(feat)
        self._check_mounts()

    def _named_paths(self, named: dict) -> None:
        """A path given by name is a part's edge, or several stretches of one end to end, `outset` metres
        outside it. `open_at` leaves an opening `gap` metres wide at each place it names: the stretches
        between become features `<id>.0`, `<id>.1`, and each opening is a place `<id>.gap.<n>`."""
        out = []
        for feat in self.features:
            p, fid = feat.get("path"), feat.get("id", "?")
            names = [p] if isinstance(p, str) else p if isinstance(p, list) and p and isinstance(p[0], str) else None
            if names is None and "open_at" not in feat:
                out.append(feat)
                continue
            if names is not None:
                missing = [n for n in names if n not in named]
                if missing:
                    self.diag.error("frame.unknown", fid, f"`path` names '{missing[0]}', which no part offers",
                                    hint="a seam part offers `<part>.edge` and its sides `<part>.edge.<n>`")
                    continue
                pts = [q for n in names for q in named[n]]
                line = LineString([q for k, q in enumerate(pts) if k == 0 or q != pts[k - 1]])
            else:
                line = LineString(p)
            if feat.get("outset"):
                line = line.offset_curve(-float(feat["outset"]))      # outward: an edge runs with its part on the left
            cuts = []
            for n, ref in enumerate(feat.get("open_at", [])):
                at = self.mounts.get(ref) if isinstance(ref, str) else ref
                way = next((f for f in self.features if f.get("id") == ref), None)
                if way is not None:      # a path of this world: the opening is where it crosses
                    ends = [self.mounts.get(way.get(k)) for k in ("starts_at", "ends_at")]
                    pts = ([ends[0]] if ends[0] else []) + [tuple(q) for q in way.get("path", [])] + ([ends[1]] if ends[1] else [])
                    hit = line.intersection(LineString(pts)) if len(pts) > 1 else None
                    at = None if hit is None or hit.is_empty else hit.representative_point().coords[0]
                if at is None:
                    self.diag.error("mount.unknown", fid, f"`open_at` names '{ref}', which is neither a place a part offers nor a path that crosses this one")
                    continue
                s, half = line.project(Point(at)), float(feat.get("gap", 12.0)) / 2
                q = line.interpolate(s)
                cuts.append((s - half, s + half))
                self.gaps[f"{fid}.gap.{n}"] = (q.x, q.y, 2 * half)
                self.mounts[f"{fid}.gap.{n}"] = (q.x, q.y)
            a, k = 0.0, 0
            for lo, hi in sorted(cuts) + [(line.length, line.length)]:
                if lo - a > 2.0:
                    piece = {key: v for key, v in feat.items() if key not in ("open_at", "outset")}
                    piece.update(id=f"{fid}.{k}" if cuts else fid, path=[list(c) for c in substring(line, a, lo).coords], piece_of=fid)
                    out.append(piece)
                    k += 1
                a = max(a, hi)
        self.features = out

    def is_late(self, feat: dict) -> bool:
        """Laid on the joined ground, after the parts have brought theirs: said on the feature, or on its layer."""
        return feat.get("after") == "parts" or feat.get("_layer") in self.late_layers

    def late(self, name: str) -> list:
        """The effects on one field of the features that are laid after the parts, in the usual order."""
        spec = self.doc.get("field", {}).get(name, {})
        tiers = {"derived": 0, "shaped": 1, "anchored": 2}
        jobs = []
        for k_i, (kind_name, kind) in enumerate(self.kinds.items()):
            for eff in kind.get("effect", []):
                if self._targets(eff, name, spec):
                    jobs += [(tiers.get(f.get("authority", kind.get("authority", "shaped")), 1), int(f.get("order", eff.get("order", 0))), k_i, f_i, f, kind, eff)
                             for f_i, f in enumerate(self.features)
                             if f.get("kind") == kind_name and self.is_late(f) and f.get("id") in self.shapes]
        return [j[4:] for j in sorted(jobs, key=lambda j: j[:4])]

    def instance_mounts(self, asset_id: str, x: float, y: float, yaw: float, scale: float = 1.0) -> dict:
        """Where an instance's mounts are in the world. A mount is declared on the asset as
        `[right, forward]` from its pivot; forward is the way its front looks."""
        out = {}
        f = (math.cos(math.radians(yaw)), math.sin(math.radians(yaw)))
        for name, m in self.pack["asset"].get(asset_id, {}).get("mount", {}).items():
            right, fwd = float(m["at"][0]) * scale, float(m["at"][1]) * scale
            out[name] = (round(x + right * f[1] + fwd * f[0], 3), round(y - right * f[0] + fwd * f[1], 3))
        return out

    def mount(self, ref: str, what: str, quiet: bool = False):
        """The world position of `owner.name`: an end of a path (`start`, `end`) or a mount an
        asset declares on an instance. None, with a diagnostic unless `quiet`, when it is not known."""
        if ref in self.mounts:
            return self.mounts[ref]
        owner, _, name = str(ref).rpartition(".")
        shape = self.shapes.get(owner)
        named = self.shapes.get(str(ref))
        if named is None:                                      # a point placed `on` something is a place too, once its host is
            feat = next((f for f in self.features if f.get("id") == ref and "on" in f and "path" not in f and "region" not in f), None)
            if feat is not None and feat["on"].get("feature") in self.shapes and feat["on"].get("feature") not in self.derived_ids:
                self._resolve_on(feat)
                self._on_early.add(ref)
                named = self.shapes.get(str(ref))
        if named is not None and named.point is not None:      # a point feature is a place by its own id
            got = (float(named.point[0]), float(named.point[1]))
        elif shape is not None and shape.path is not None and name in ("start", "end"):
            got = tuple(float(v) for v in shape.path.pts[0 if name == "start" else -1])
        else:
            spec = next((i for i in self.doc.get("instance", []) if i.get("id") == owner), None)
            got = None
            if spec is not None and not ("on" in spec and spec["on"].get("feature") not in self.shapes):
                from .placement import resolve_position      # placement builds on this module
                pose = resolve_position(self, spec, owner)
                if pose is not None:
                    got = self.instance_mounts(spec.get("asset", ""), pose[0], pose[1], pose[2], float(spec.get("scale", 1.0))).get(name)
            if got is None:
                if not quiet:
                    known = sorted(self.pack["asset"].get(spec.get("asset", ""), {}).get("mount", {})) if spec else []
                    self.diag.error("mount.unknown", what, f"'{ref}' is not a place that can be found"
                                    + (f"; '{owner}' has: {', '.join(known) or 'no mounts'}" if spec else
                                       "; an end of a path is written `id.start` or `id.end`"))
                return None
        self.mounts[ref] = got
        return got

    def _check_mounts(self) -> None:
        """A mount may say it expects a path. An instance whose door no path reaches is reported."""
        ends = [tuple(sh.path.pts[k]) for sh in self.shapes.values() if sh.path is not None for k in (0, -1)]
        for spec in self.doc.get("instance", []):
            asset = self.pack["asset"].get(spec.get("asset", ""), {})
            wants = {n: m for n, m in asset.get("mount", {}).items() if m.get("expects") == "path"}
            if not wants or ("on" in spec and spec["on"].get("feature") not in self.shapes):
                continue
            for name in wants:
                at = self.mount(f"{spec['id']}.{name}", spec["id"], quiet=True)
                if at is not None and not any(math.hypot(at[0] - e[0], at[1] - e[1]) < 2.0 for e in ends):
                    self.diag.warn("mount.unmet", spec["id"], f"no path reaches its {name}", where=at,
                                   hint=f'end a path there: ends_at = "{spec["id"]}.{name}"')

    def frame_at(self, on: dict, what: str):
        """Position, tangent and outward-or-left normal at a place in another feature's frame."""
        host = self.shapes.get(on.get("feature", ""))
        line = None if host is None else (host.path or host.outline)
        if line is None:
            self.diag.error("frame.unknown", what, f"`on` needs a path or a region; '{on.get('feature')}' is neither")
            return None
        p, tan, left = line.at(float(on.get("t", 0.5)))
        normal = -left if host.type == "region" else left      # for a region, offset is outward
        return p, tan, normal

    def _resolve_on(self, feat: dict) -> None:
        got = self.frame_at(feat["on"], feat.get("id", "?"))
        if got is None:
            return
        p, tan, normal = got
        p = p + normal * float(feat["on"].get("offset", 0.0))
        feat["at"] = [float(p[0]), float(p[1])]
        self._add_shape(feat)

    def derive(self, only: str | None = None, waiting: bool = False) -> None:
        """Features computed from a field. A flood is the cells below a level that connect to a point.

        Each flood is derived once, from the field it reads, which is resolved first if need be.
        `only` derives one flood by id. `waiting` leaves out a flood whose field is being resolved
        right now: that flood has to wait for its field, and must not hold back floods that read
        other fields. Features placed `on` a flood follow as soon as their host has been tried."""
        for feat in self.features:
            if ("flood" not in feat and "where" not in feat) or feat.get("id") in self._flooded or (only is not None and feat.get("id") != only):
                continue
            if "where" in feat:                     # a region that is wherever an expression holds
                if waiting and self._resolving:
                    continue
                self._flooded.add(feat.get("id"))
                self._where(feat)
                continue
            if waiting and feat["flood"].get("field", "height") in self._resolving:
                continue
            self._flooded.add(feat.get("id"))       # marked first: flooding resolves a field, which may ask again
            self._flood(feat)
        for feat in [f for f in self.deferred if f["on"].get("feature") in self._flooded]:
            self.deferred.remove(feat)
            self._resolve_on(feat)

    def _where(self, feat: dict) -> None:
        """`where = "<expression>"` makes a region of the largest connected piece of ground where the
        expression holds, holes filled. It may read fields and other features, so it moves when they do."""
        fid, g = feat["id"], self.grid
        held = self.evaluate(feat["where"], self.env(), (g.ny, g.nx), f"feature '{fid}' where") > 0.5
        lab, n = label(held)
        if not n:
            self.diag.error("feature.where_empty", fid, "its `where` holds nowhere in this world, so it has no shape",
                            hint="loosen the expression; it is evaluated at every post and must be true somewhere")
            return
        sizes = np.bincount(lab.ravel())[1:]
        mask = binary_fill_holes(lab == 1 + int(np.argmax(sizes)))
        pad = np.pad(mask.astype(np.float32), 1)
        xs = g.x0 + (np.arange(g.nx + 2) - 1) * g.res
        ys = g.y0 + (np.arange(g.ny + 2) - 1) * g.res
        loops = [l for l in contour_generator(x=xs, y=ys, z=pad).lines(0.5) if len(l) > 8]
        shore = max(loops, key=lambda l: abs(float(np.sum(l[:-1, 0] * l[1:, 1] - l[1:, 0] * l[:-1, 1]))))
        keep = list(Polygon(shore).simplify(2.0 * g.res).exterior.coords)[:-1]      # a post's width of stair is not worth a vertex
        feat["region"] = [[round(float(x), 1), round(float(y), 1)] for x, y in keep]
        feat["_mask"] = mask
        self._add_shape(feat)

    def _flood(self, feat: dict) -> None:
        fid, spec, g = feat["id"], feat["flood"], self.grid
        source = spec.get("field", "height")
        if source not in self.doc.get("field", {}):
            self.diag.error("field.unknown", fid, f"its flood reads the field '{source}', which is not declared")
            return
        h = self.field(source)
        x, y = feat["at"]
        fence = float(spec.get("fence", 120.0))
        win = g.window((x, y, x, y), fence + 4.0)
        H, X, Y = h[win], g.X[win], g.Y[win]
        held = np.hypot(X - x, Y - y) <= fence
        sj = int(np.clip(round((y - g.y0) / g.res) - win[0].start, 0, H.shape[0] - 1))
        si = int(np.clip(round((x - g.x0) / g.res) - win[1].start, 0, H.shape[1] - 1))
        seed_h = float(H[sj, si])

        def body(level):
            lab, _ = label(H < level)
            comp = lab == lab[sj, si] if lab[sj, si] else np.zeros_like(held)
            return comp, bool((comp & ~held).any())

        level = spec.get("level", "spill")
        if level == "spill":           # the highest level the ground holds inside the fence
            lo, hi = seed_h + 0.05, seed_h + float(spec.get("max_depth", 8.0))
            if body(lo)[1]:
                self.diag.error("water.no_basin", fid, "the ground here holds no water inside the fence",
                                where=(x, y), hint="shape a basin under it, widen `fence`, or move the point")
                return
            for _ in range(22):
                mid = 0.5 * (lo + hi)
                if body(mid)[1]:
                    hi = mid
                else:
                    lo = mid
            level = lo - float(spec.get("freeboard", 0.3))
        level = float(level)
        comp, leaks = body(level)
        if leaks:
            self.diag.warn("water.leaks", fid, f"at level {level:.1f} m the water runs past the fence of {fence:g} m",
                           where=(x, y), hint="lower the level, or use `level = \"spill\"`")
            comp &= held
        if not comp.any():
            self.diag.error("water.dry", fid, f"level {level:.1f} m is below the ground at the point ({seed_h:.1f} m)", where=(x, y))
            return
        loops = contour_generator(x=X[0], y=Y[:, 0], z=np.where(comp | ~held, H, level + 1.0)).lines(level)
        loops = [l for l in loops if len(l) > 8 and np.allclose(l[0], l[-1])]
        if not loops:
            self.diag.error("water.dry", fid, "the flooded area has no closed shore", where=(x, y))
            return

        def area(l):
            return 0.5 * abs(float(np.sum(l[:-1, 0] * l[1:, 1] - l[1:, 0] * l[:-1, 1])))

        shore = max(loops, key=area)
        mask = np.zeros((g.ny, g.nx), dtype=bool)
        mask[win] = comp
        feat["region"] = shore[:-1].tolist()
        feat["_mask"] = mask
        feat["level"] = level
        self._add_shape(feat)

    def _add_shape(self, feat: dict) -> None:
        fid = feat.get("id")
        if not fid:
            self.diag.error("feature.no_id", "?", "a feature has no id")
            return
        if fid in self.shapes:
            self.diag.error("feature.duplicate", fid, f"two features share the id '{fid}'")
            return
        if self.kinds.get(feat.get("kind", ""), {}).get("plan") is False:
            return      # a kind with no plan: its features live wholly in their attributes
        try:
            shape = Shape(self, feat)
        except ValueError as exc:
            self.diag.error("feature.shape", fid, str(exc))
            return
        g = self.grid
        leaves = self.kinds.get(feat.get("kind", ""), {}).get("exit")      # an exit stands on the edge or just past it
        if not leaves and (shape.bbox[2] < g.x0 or shape.bbox[0] > g.x1 or shape.bbox[3] < g.y0 or shape.bbox[1] > g.y1):
            self.diag.warn("feature.outside", fid, "the feature lies wholly outside the world extent")
        if feat.get("kind") and feat["kind"] not in self.kinds:
            self.diag.info("kind.passthrough", fid,
                           f"kind '{feat['kind']}' is not declared; the feature is passed through untouched")
        self.shapes[fid] = shape

    def add_feature(self, feat: dict) -> None:
        self.features.append(feat)
        self._add_shape(feat)

    def attrs(self, feat: dict) -> dict:
        """Kind defaults overlaid by the feature's own attributes."""
        out = {k: v for k, v in self.kinds.get(feat.get("kind", ""), {}).get("defaults", {}).items() if not isinstance(v, str)}
        out.update({k: v for k, v in feat.items() if isinstance(v, (int, float)) and not isinstance(v, bool)})
        return out

    # ---- expressions ------------------------------------------------------------------------------

    def env(self, win=None, extra: dict | None = None) -> dict:
        g = self.grid
        win = win or (slice(0, g.ny), slice(0, g.nx))
        X, Y = g.X[win], g.Y[win]

        def fbm(wavelength, octaves=4, seed=0, gain=0.5, lacunarity=2.0):
            total, amp, norm, freq = 0.0, 1.0, 0.0, 1.0 / wavelength
            for o in range(int(octaves)):
                total = total + amp * _perlin(X * freq, Y * freq, self.seed * 1000 + seed * 31 + o)
                norm += amp
                amp *= gain
                freq *= lacunarity
            return total / norm

        def ridged(wavelength, octaves=4, seed=0, gain=0.5, lacunarity=2.0):
            total, amp, norm, freq = 0.0, 1.0, 0.0, 1.0 / wavelength
            for o in range(int(octaves)):
                n = 1.0 - np.abs(_perlin(X * freq, Y * freq, self.seed * 1000 + seed * 31 + o + 500))
                total = total + amp * n * n
                norm += amp
                amp *= gain
                freq *= lacunarity
            return total / norm

        def f(name):
            return self.field(name)[win]

        def slope(name):
            """Slope of a scalar field in degrees."""
            arr = self.field(name)
            gy, gx = np.gradient(arr, g.res)
            return np.degrees(np.arctan(np.hypot(gx, gy)))[win]

        def _shape(fid):
            if fid not in self.shapes and fid in self.derived_ids:
                self.derive(only=fid)
            if fid not in self.shapes:
                raise Unresolved("feature.unknown", f"no feature '{fid}'")
            return self.shapes[fid].frame(1e9)

        def _all(fid, key):                         # 'kind:<name>' is every feature of that kind, as one shape
            ids = [f["id"] for f in self.features if f.get("kind") == fid[5:]] if fid.startswith("kind:") else [fid]
            if not ids:
                raise Unresolved("feature.unknown", f"no feature of kind '{fid[5:]}'")
            out = _shape(ids[0])[key]
            for i in ids[1:]:
                out = np.minimum(out, _shape(i)[key])
            return out[win]

        def dist(fid):
            return _all(fid, "d")

        def sdist(fid):
            return _all(fid, "sd")

        def along(fid):
            return _shape(fid)["t"][win]

        def near(fid, radius):
            return 1.0 - smoothstep(0.0, radius, dist(fid))

        def inside(pid):
            """How much of the ground is a part's: 0 outside its footprint, 1 an apron inside it."""
            if pid not in self.part_w:
                raise Unresolved("part.unknown", f"no part '{pid}' has been joined where this is read")
            return self.part_w[pid][win]

        def outside(pid):
            """Metres outside a part's footprint; 0 inside it."""
            inside(pid)
            if pid not in self._part_d:
                self._part_d[pid] = (distance_transform_edt(self.part_w[pid] <= 0) * g.res).astype(np.float32)
            return self._part_d[pid][win]

        env = {
            "__builtins__": {}, "x": X, "y": Y, "pi": math.pi,
            "sin": np.sin, "cos": np.cos, "abs": np.abs, "sqrt": np.sqrt, "exp": np.exp, "pow": np.power,
            "min": np.minimum, "max": np.maximum, "clamp": np.clip, "where": np.where,
            "smoothstep": smoothstep, "mix": lambda a, b, t: a + (b - a) * t,
            "fbm": fbm, "ridged": ridged, "f": f, "slope": slope,
            "dist": dist, "sdist": sdist, "along": along, "near": near, "inside": inside, "outside": outside,
        }
        env.update(self.params)
        if extra:
            env.update(extra)
        return env

    def evaluate(self, expr, env: dict, shape, what: str) -> np.ndarray:
        if isinstance(expr, (int, float)):
            return np.full(shape, float(expr), dtype=np.float32)
        try:
            val = eval(" ".join(expr.split()), env)  # spike shortcut: the rewrite needs a real expression language
        except RecursionError:
            raise
        except Unresolved as exc:       # the expression is sound; a name in it has no value
            self.report_unresolved(exc, what, expr)
            return np.zeros(shape, dtype=np.float32)
        except Exception as exc:
            self.diag.error("expr.invalid", what, f"cannot evaluate `{' '.join(str(expr).split())[:90]}`: {exc}")
            return np.zeros(shape, dtype=np.float32)
        return np.broadcast_to(np.asarray(val, dtype=np.float32), shape).astype(np.float32, copy=True)

    def report_unresolved(self, exc: Unresolved, what: str, expr) -> None:
        """Say what an expression could not name. The compile goes on with the expression taken as
        zero, as it does for any expression that fails, so one mistake does not hide the others.
        A cycle is met again by every expression that reads into it, so it is reported only once."""
        text = " ".join(str(expr).split())[:90]
        if exc.code == "field.cycle":
            if frozenset(exc.chain) in self._cycles:
                return
            self._cycles.add(frozenset(exc.chain))
            self.diag.error("field.cycle", exc.chain[0], " -> ".join(exc.chain) + f"; {what} closes the cycle, "
                            f"so its expression `{text}` is taken as 0", hint="one of these fields must stop reading the other")
        else:
            self.diag.error(exc.code, what, f"`{text}` names something that is not declared: {exc}")

    # ---- fields -----------------------------------------------------------------------------------

    def field(self, name: str) -> np.ndarray:
        if name in self.fields:
            return self.fields[name]
        specs = self.doc.get("field", {})
        if name not in specs:
            raise Unresolved("field.unknown", f"no field '{name}'")
        if name in self._resolving:
            chain = self._resolving[self._resolving.index(name):] + [name]
            raise Unresolved("field.cycle", "field cycle: " + " -> ".join(chain), chain)
        # The name comes off the stack however resolution ends. Left behind after a failure, it would
        # make the next lookup of this field report a cycle that is not there.
        self._resolving.append(name)
        try:
            arr = self._resolve(name, specs[name])
        finally:
            self._resolving.pop()
        self.fields[name] = arr
        return arr

    def _resolve(self, name: str, spec: dict) -> np.ndarray:
        """Work one field out: its base and layers, then every effect that targets it."""
        g = self.grid
        full = (g.ny, g.nx)
        if spec.get("type") == "material":
            names = []
            for layer in spec.get("layers", []):
                if layer["paint"] not in names:
                    names.append(layer["paint"])
            # A kind brings its paint only when some feature is of that kind. Kinds come from shared
            # libraries, and one this world never uses must not take one of the eight material planes.
            used = {f.get("kind") for f in self.features}
            for kind_name, kind in self.kinds.items():
                for eff in kind.get("effect", []):
                    if (kind_name in used and self._targets(eff, name, spec) and "paint" in eff
                            and eff["paint"] not in names):
                        names.append(eff["paint"])
            self.materials = names
            arr = np.zeros((len(names),) + full, dtype=np.float32)
            env = self.env()
            for key, expr in spec.get("let", {}).items():
                env[key] = self.evaluate(expr, env, full, f"field.{name}.let.{key}")
            for layer in spec.get("layers", []):
                mask = np.clip(self.evaluate(layer.get("mask", 1.0), env, full, f"field.{name}"), 0, 1)
                arr *= (1.0 - mask)[None]
                arr[names.index(layer["paint"])] += mask
        else:
            env = self.env()
            lets = {}
            for key, expr in spec.get("let", {}).items():
                lets[key] = env[key] = self.evaluate(expr, env, full, f"field.{name}.let.{key}")
            if "file" in spec:
                arr = self._from_file(name, spec)
            else:
                arr = self.evaluate(spec.get("base", 0.0), env, full, f"field.{name}")
            for layer in spec.get("layers", []):
                env = self.env(extra={"v": arr, **lets})
                val = self.evaluate(layer["value"], env, full, f"field.{name}")
                mask = np.clip(self.evaluate(layer.get("mask", 1.0), env, full, f"field.{name}"), 0, 1)
                arr = arr + (val - arr) * mask
        # effects, by authority: derived features conform to the world, shaped ones shape it, and the
        # world conforms to anchored ones, which are therefore applied last. Order inside a tier is
        # declaration order, except that an anchor named in another's `over` goes first.
        tiers = {"derived": 0, "shaped": 1, "anchored": 2}
        # A flood that reads another field has effects of its own on this one, so it has to exist now.
        # One that reads this field cannot: it is derived from the finished field, and conforms to it.
        if any(f.get("flood", {}).get("field", "height") != name for f in self.features if "flood" in f):
            self.derive(waiting=True)
        jobs = []
        for k_i, (kind_name, kind) in enumerate(self.kinds.items()):
            for eff in kind.get("effect", []):
                if not self._targets(eff, name, spec):
                    continue
                for f_i, feat in enumerate(self.features):
                    if feat.get("kind") != kind_name or self.is_late(feat):
                        continue
                    auth = feat.get("authority", kind.get("authority", "shaped"))
                    if "flood" in feat:
                        auth = "derived"
                        if feat["flood"].get("field", "height") == name:
                            self.diag.info("authority.conforms", feat["id"],
                                           f"it is derived from '{name}', so its effect on '{name}' is not applied")
                            continue
                    if auth not in tiers:
                        self.diag.error("authority.unknown", feat["id"], f"authority '{auth}' is not derived, shaped or anchored")
                        continue
                    if feat["id"] in self.shapes:
                        # among anchors the more specific wins: a building on a terrace is applied after
                        # the terrace, so it takes the terrace's level instead of fighting it
                        shp = self.shapes[feat["id"]]
                        area = 0.0
                        if tiers[auth] == 2 and shp.polygon is not None:
                            q = np.asarray(shp.polygon)
                            area = 0.5 * abs(float(np.sum(q[:, 0] * np.roll(q[:, 1], -1) - np.roll(q[:, 0], -1) * q[:, 1])))
                        # `order` on an effect states its place among all effects of its tier, whatever the
                        # feature: every way lays its pavement (0) before any lays its roadway (1), so
                        # one street's pavement never lies across another's roadway where they meet
                        # a feature may state its own place in the sequence, which then stands for all its effects
                        jobs.append((tiers[auth], int(feat.get("order", eff.get("order", 0))), len(feat.get("over", [])), -area, k_i, f_i, feat, kind, eff))
        jobs.sort(key=lambda j: j[:6])
        held: list = []
        for tier, _, _, _, _, _, feat, kind, eff in jobs:
            got = self._apply(name, arr, feat, kind, eff)
            if tier == 2 and got is not None and spec.get("unit"):      # only measured fields can disagree by an amount
                self._meet(name, feat, got, held)
                held.append((feat, got))
        return arr

    def _from_file(self, name: str, spec: dict) -> np.ndarray:
        """A field someone else worked out. Two forms, told apart by the file's suffix.

        `.f32` is raw 32-bit floats, one per post, south row first: what a step hands back. It must
        match the grid exactly and is taken as it is.

        Anything else is a picture the author already has (a heightmap, a noise tile, a painted
        mask). It is the author's to get right; the compiler only places and scales it:
          channel   which channel to read: r, g, b, a or luma (the default)
          range     the values black and white stand for; default [0, 1]
          extent    where the picture lies in the world, [x0, y0, x1, y1]; default the whole world
          outside   the value beyond `extent`; default: the picture's edge carries on
          filter    nearest, linear (the default) or cubic
        The picture's top row is north. It is resampled to the grid whatever its size."""
        g = self.grid
        full = (g.ny, g.nx)
        file = Path(spec["file"])
        if not file.is_file():
            self.diag.error("field.file", name, f"{file.name} does not exist", hint=str(file))
            return np.zeros(full, np.float32)
        if file.suffix == ".f32":
            raw = np.fromfile(file, dtype=np.float32)
            if raw.size != g.ny * g.nx:
                self.diag.error("field.file", name, f"{file.name} holds {raw.size:,} values; the grid has "
                                f"{g.nx} x {g.ny} = {g.nx * g.ny:,} posts", hint="a .f32 field file is float32, one value per post")
                return np.zeros(full, np.float32)
            return raw.reshape(full).astype(np.float32)
        try:
            img = Image.open(file)
            img.load()
        except Exception as exc:      # not a picture this library reads
            self.diag.error("field.file", name, f"{file.name} could not be read as a picture: {exc}")
            return np.zeros(full, np.float32)
        channel = spec.get("channel", "luma")
        if img.mode in ("I;16", "I;16B", "I;16L", "I", "F"):      # one deep channel
            px = np.asarray(img, dtype=np.float64)
            px = px / 65535.0 if img.mode != "F" else px
            depth = 65536 if img.mode != "F" else 0
        else:
            if channel == "luma":
                px = np.asarray(img.convert("L"), dtype=np.float64) / 255.0
            elif channel in "rgba" and len(channel) == 1:
                bands = img.convert("RGBA").split()
                px = np.asarray(bands["rgba".index(channel)], dtype=np.float64) / 255.0
            else:
                self.diag.error("field.file", name, f"channel '{channel}' is not one of r, g, b, a, luma")
                return np.zeros(full, np.float32)
            depth = 256
        lo, hi = map(float, spec.get("range", [0.0, 1.0]))
        if depth == 256 and spec.get("unit") and abs(hi - lo) / 255.0 > 0.1:
            self.diag.warn("field.image_coarse", name, f"{file.name} has 256 levels over {lo:g} to {hi:g} {spec['unit']}: "
                           f"each level is {abs(hi - lo) / 255.0:.2f} {spec['unit']}, which shows as steps",
                           hint="use a 16-bit picture, or keep an 8-bit one for masks")
        x0, y0, x1, y1 = map(float, spec.get("extent", [g.x0, g.y0, g.x1, g.y1]))
        rows, cols = px.shape
        # pixel centres: column 0 at x0, the last at x1; row 0 (the top) at y1
        cx = (g.X - x0) / max(x1 - x0, 1e-9) * (cols - 1)
        cy = (y1 - g.Y) / max(y1 - y0, 1e-9) * (rows - 1)
        order = {"nearest": 0, "linear": 1, "cubic": 3}.get(spec.get("filter", "linear"), 1)
        arr = lo + (hi - lo) * map_coordinates(px, [cy, cx], order=order, mode="nearest")
        if "outside" in spec:
            beyond = (g.X < x0) | (g.X > x1) | (g.Y < y0) | (g.Y > y1)
            arr = np.where(beyond, float(spec["outside"]), arr)
        return arr.astype(np.float32)

    def _meet(self, name: str, feat: dict, got: tuple, held: list) -> None:
        """Two anchors on one field. Their cores may not overlap; where only their aprons do, there is
        no free ground between them to take up the difference, so the step that remains is reported."""
        win, mask, val = got
        g = self.grid
        for other, (owin, omask, oval) in held:
            if other["id"] == feat["id"]:
                continue
            j0, j1 = max(win[0].start, owin[0].start), min(win[0].stop, owin[0].stop)
            i0, i1 = max(win[1].start, owin[1].start), min(win[1].stop, owin[1].stop)
            if j0 >= j1 or i0 >= i1:
                continue
            a = (slice(j0 - win[0].start, j1 - win[0].start), slice(i0 - win[1].start, i1 - win[1].start))
            b = (slice(j0 - owin[0].start, j1 - owin[0].start), slice(i0 - owin[1].start, i1 - owin[1].start))
            both = (mask[a] > 0.05) & (omask[b] > 0.05)
            if not both.any():
                continue
            gap = np.where(both, np.abs(val[a] - oval[b]), 0.0)
            if float(gap.max()) < 0.05:
                continue
            j, i = np.unravel_index(int(gap.argmax()), gap.shape)
            where = (g.x0 + (i0 + i) * g.res, g.y0 + (j0 + j) * g.res)
            declared = other["id"] in feat.get("over", []) or feat["id"] in other.get("over", [])
            cores = bool(((mask[a] > 0.5) & (omask[b] > 0.5) & (gap > 0.05)).any())
            if cores and not declared:
                self.diag.error("authority.conflict", feat["id"],
                                f"it and '{other['id']}' are both anchored and both claim '{name}' here, {gap.max():.1f} m apart",
                                where=where, hint=f"move one, or say which wins: over = [\"{other['id']}\"]")
            elif not declared:
                self.diag.warn("authority.squeezed", feat["id"],
                               f"its apron meets the apron of '{other['id']}' with {gap.max():.1f} m of '{name}' between them and no free ground",
                               where=where, hint="move them apart, narrow an apron, or say which wins with `over`")

    @staticmethod
    def _targets(eff: dict, name: str, spec: dict) -> bool:
        target = eff.get("field", "")
        return target == name or (target.startswith("#") and target[1:] in spec.get("tags", []))

    def _apply(self, name: str, arr: np.ndarray, feat: dict, kind: dict, eff: dict):
        """One effect of one feature on one field: v = mix(v, value, mask) or paint a material by mask."""
        fid = feat["id"]
        shape = self.shapes[fid]
        attrs = self.attrs(feat)
        reach = float(eff.get("reach", kind.get("reach", 48.0)))
        fr = shape.frame(reach)
        win = fr["win"]
        is_material = arr.ndim == 3
        v = arr[(slice(None),) + win] if is_material else arr[win]
        wshape = v.shape[-2:]
        if wshape[0] == 0 or wshape[1] == 0:
            return
        g = self.grid
        local = dict(attrs)
        local.update({"d": fr["d"], "sd": fr["sd"], "t": fr.get("t", 0.0)})
        if not is_material:
            local["v"] = v
            if shape.type == "path":
                prof = g.sample(arr, shape.path.pts[:, 0], shape.path.pts[:, 1])
                window = max(3, int(float(attrs.get("grade", 24.0)) / 0.25))
                prof = uniform_filter1d(prof, size=window, mode="nearest")
                prof = uniform_filter1d(prof, size=window, mode="nearest")
                local["profile"] = prof[fr["idx"]].astype(np.float32)
                grade = np.abs(np.gradient(prof, 0.25)) * 100.0
                limit = attrs.get("max_grade")
                if limit is not None and grade.max() > limit:
                    k = int(grade.argmax())
                    self.diag.warn("seam.grade", fid,
                                   f"{feat['kind']} climbs at {grade.max():.0f}% after grading; its limit is {limit:.0f}%",
                                   where=shape.path.pts[k], hint="lengthen the route, raise `grade`, or accept a steeper limit")
                below = prof
            elif shape.type == "region":
                below = v[fr["sd"] <= 0]
            else:
                below = g.sample(arr, [shape.point[0]], [shape.point[1]])
            if below.size:
                local.update({"base": float(np.mean(below)), "base_min": float(np.min(below)),
                              "base_max": float(np.max(below))})
            else:
                local.update({"base": 0.0, "base_min": 0.0, "base_max": 0.0})

        def under(other, how="mean"):
            """A resolved field measured over this feature: mean, min, max, or the lowest point of its edge."""
            arr2 = self.field(other)[win]
            if shape.type == "region":
                pick = arr2[np.abs(fr["sd"]) <= g.res * 1.5] if how == "edge_min" else arr2[fr["sd"] <= 0]
            elif shape.type == "path":
                pick = g.sample(self.field(other), shape.path.pts[:, 0], shape.path.pts[:, 1])
            else:
                pick = g.sample(self.field(other), [shape.point[0]], [shape.point[1]])
            if pick.size == 0:
                return 0.0
            return float({"mean": np.mean, "min": np.min, "max": np.max, "edge_min": np.min}[how](pick))

        local["under"] = under
        env = self.env(win, local)
        what = f"kind.{feat['kind']}"
        for key, expr in kind.get("let", {}).items():
            try:      # a kind may name things that only exist for another of its fields (a volume's z)
                env[key] = np.broadcast_to(np.asarray(eval(" ".join(str(expr).split()), env), dtype=np.float32), wshape)
            except Unresolved as exc:
                self.report_unresolved(exc, f"{what}.let.{key}", expr)
            except NameError:
                pass
        mask = np.clip(self.evaluate(eff.get("mask", 1.0), env, wshape, what), 0, 1)
        mask = np.where(fr["d"] > reach, 0.0, mask).astype(np.float32)
        edge = np.concatenate([mask[0], mask[-1], mask[:, 0], mask[:, -1]])
        touches_world_edge = (win[0].start == 0 or win[0].stop == g.ny or win[1].start == 0 or win[1].stop == g.nx)
        if edge.max() > 0.02 and not touches_world_edge:
            self.diag.warn("effect.unbounded", fid,
                           f"the mask of {feat['kind']} on '{name}' is still {edge.max():.2f} at its reach of {reach:g} m; "
                           "expect a visible edge", hint="make the mask fall to zero, or raise `reach`")
        if is_material:
            v *= (1.0 - mask)[None]
            v[self.materials.index(eff["paint"])] += mask
            return
        val = self.evaluate(eff["value"], env, wshape, what)
        change = (val - v) * mask
        limit_key = eff.get("max_change")
        if limit_key is not None:
            limit = float(eval(limit_key, {"__builtins__": {}}, attrs) if isinstance(limit_key, str) else limit_key)
            if "check_where" in eff:      # the limit applies only where this holds
                change_checked = np.where(self.evaluate(eff["check_where"], env, wshape, what) > 0, change, 0.0)
            else:
                change_checked = change
            worst = float(np.abs(change_checked).max()) if change.size else 0.0
            if worst > limit:
                j, i = np.unravel_index(int(np.abs(change_checked).argmax()), change.shape)
                where = (g.X[win][j, i], g.Y[win][j, i])
                self.diag.error("seam.change_exceeds", fid,
                                f"{feat['kind']} would move '{name}' by {worst:.1f} m; its limit is {limit:.1f} m",
                                where=where, hint="move it to gentler ground, or raise the limit on purpose")
        v += change
        return win, mask, val
