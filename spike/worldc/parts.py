"""Parts: a world placed inside another world as a unit.

A part is compiled by itself first, in its own frame and with its own seed, so it is the same thing
wherever it lands. It then meets the master at a boundary, which is one of two kinds:

  seam    the part brings terrain. Inside its footprint the master's fields become the part's,
          blended over an apron. The compiler lifts the part to sit on the land it joins and reports
          how big a step is left at its edge.
  portal  the part brings an interior. Nothing is blended. One of its doors is put at a point in the
          master, and the compiler checks the threshold and the ground under its footprint.

Names are scoped: everything a part brings is prefixed with the part's id.
"""
from __future__ import annotations

import copy
import hashlib
import json
import tomllib
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from shapely.geometry import LineString, Point, Polygon
from scipy.ndimage import distance_transform_edt, gaussian_filter

from .diagnostics import Diagnostics
from .geometry import smoothstep
from .world import Grid, World


def load_parts(master_doc: dict, master_path: Path, pack_path: Path, diag: Diagnostics, master_pack: dict) -> list[dict]:
    """Compile every part the master names, alone, and work out where each one lands. A part uses
    the master's pack unless its row names its own."""
    from .pipeline import compile_world      # imported here: the pipeline loads parts, and a part is compiled by the pipeline

    parts = []
    early: dict = {}
    for spec in master_doc.get("part", []):
        own_pack = (master_path.parent / spec["pack"]).resolve() if "pack" in spec else pack_path
        given, outline = hand_down(master_doc, master_path, spec, master_pack, diag, early)
        done = compile_world((master_path.parent / spec["file"]).resolve(), pack_path=own_pack, as_part=True,
                             variants=list(spec.get("variants", [])), given=given)
        sub = {"doc": done.doc, "world": done.world, "instances": done.instances, "built": done.built,
               "volumes": done.volumes}
        for table in ("asset", "surface", "ground"):      # what the part's pack brings, the master can now place
            for key, val in sub["world"].pack.get(table, {}).items():
                master_pack.setdefault(table, {}).setdefault(key, val)
        diag.adopt(done.diag, spec["id"])       # a part's own diagnostics travel with its name on them
        if not done.sound:                      # a field of it did not resolve, and the line above has said which
            continue
        # A seam part's ground replaces the master's inside its footprint, so it must have one.
        # A portal part brings an interior only and blends with nothing.
        if "portal" not in spec and "ground" not in done.world.fields:
            diag.error("field.required", spec["id"], "a seam part brings terrain, so it needs a `ground` field; the part is left out",
                       hint="declare `[field.ground]` in the part, or hang it on a door with `portal`")
            continue
        sub["spec"] = spec
        sub["id"] = spec["id"]
        pw = sub["world"]
        if "portal" in spec:
            door = next((f for f in pw.features if f.get("id") == spec["portal"]["door"]), None)
            if door is None or "at" not in door:
                diag.error("part.no_door", spec["id"], f"the part has no door '{spec['portal']['door']}'")
                continue
            sub["offset"] = np.asarray(spec["portal"]["at"], dtype=float) - np.asarray(door["at"], dtype=float)
            sub["door_z"] = float(door.get("floor_z", 0.0))
            rooms = [np.asarray(pw.shapes[r].polygon) for r in sub["built"].get("rooms", {}) if r in pw.shapes]
            pts = np.vstack(rooms) if rooms else np.zeros((1, 2))
            sub["rect"] = [pts[:, 0].min() - 1, pts[:, 1].min() - 1, pts[:, 0].max() + 1, pts[:, 1].max() + 1]
        else:
            sub["offset"] = np.asarray(spec["at"], dtype=float)
            foot = spec.get("footprint", [pw.grid.x0, pw.grid.y0, pw.grid.x1, pw.grid.y1])
            if isinstance(foot, str):                    # a region of the master, by name: the part keeps its whole
                sub["poly"], sub["named"] = outline, True     # box, so the ways to its exits can still be carried
                foot = [pw.grid.x0, pw.grid.y0, pw.grid.x1, pw.grid.y1]
                if outline is None:
                    continue
            elif isinstance(foot[0], (list, tuple)):      # an outline, not a box: the box is what holds it
                sub["poly"] = [[float(x), float(y)] for x, y in foot]
                foot = [min(q[0] for q in foot), min(q[1] for q in foot), max(q[0] for q in foot), max(q[1] for q in foot)]
            sub["rect"] = [float(v) for v in foot]
        parts.append(sub)
    return parts


SHAPE_KEYS = ("region", "ellipse", "path", "at", "on", "where", "flood", "authority")


def hand_down(master_doc: dict, master_path: Path, spec: dict, pack: dict, diag: Diagnostics, early: dict):
    """What a master gives a part before the part is built: `give = { <the part's name> = "<the master's>" }`
    hands down a region or a field of the master, and `footprint = "<region>"` cuts the part to one.
    Both are worked out on the master as it stands before any part is placed, since the part is built
    from them. Returns what `compile_world` takes as `given`, and the footprint's outline, both in the
    part's own frame."""
    wants, foot = spec.get("give", {}), spec.get("footprint")
    if "portal" in spec or (not wants and not isinstance(foot, str)):
        return None, None
    if "world" not in early:      # the master alone: its own terrain and features, nothing a part brings
        early["world"] = World(copy.deepcopy(master_doc), pack, Diagnostics())
    pre, (ox, oy) = early["world"], spec["at"]
    pw = tomllib.loads((master_path.parent / spec["file"]).read_text())["world"]
    pg = Grid(pw["extent"], pw.get("resolution", 0.5))

    def outline(name):
        if name in pre.derived_ids:
            pre.derive(only=name)
        shp = pre.shapes.get(name)
        if shp is None or shp.polygon is None:
            diag.error("give.unknown", spec["id"], f"it is given '{name}', which is not a region of this world",
                       hint="name a feature with `region` or `ellipse` that does not start or end at a part")
            return None
        return [[round(float(x) - ox, 2), round(float(y) - oy, 2)] for x, y in shp.polygon]

    given = {"regions": {}, "fields": {}, "features": {}, "names": sorted(wants)}
    h = hashlib.sha256()
    for name, src in sorted(wants.items()):
        if src.startswith("kind:"):       # every feature of a kind, with what it says: a set handed down whole
            rows = []
            for ft in pre.features:
                if ft.get("kind") != src[5:]:
                    continue
                pts = outline(ft["id"])
                if pts is not None:
                    rows.append({**{k: v for k, v in ft.items() if k not in SHAPE_KEYS and not k.startswith("_")}, "region": pts})
            given["features"][name] = rows
            h.update(json.dumps(rows, sort_keys=True).encode())
        elif src in master_doc.get("field", {}):
            try:
                vals = pre.grid.sample(pre.field(src), pg.X + ox, pg.Y + oy).astype("<f4")
            except Exception as exc:      # it read something only a part brings: that is a circle
                diag.error("give.early", spec["id"], f"the field '{src}' is given to this part, so it has to exist before any part is placed, and it does not: {exc}",
                           hint="a given field may read this world's own terrain and features, nothing a part brings")
                continue
            file = Path(__file__).resolve().parent.parent / "build" / ".given" / master_path.parent.name / spec["id"] / f"{name}.f32"
            file.parent.mkdir(parents=True, exist_ok=True)
            vals.tofile(file)
            given["fields"][name] = str(file)
            h.update(vals.tobytes())
        else:
            pts = outline(src)
            if pts is not None:
                given["regions"][name] = pts
                h.update(json.dumps(pts).encode())
    given["key"] = h.hexdigest()[:10]
    return (given if wants else None), (outline(foot) if isinstance(foot, str) else None)


def take_given(doc: dict, given: dict | None, diag: Diagnostics) -> None:
    """A world may declare what it can be given: `[given.<name>]` with `type = "region"` (default: the
    world's whole extent) or `type = "field"` (default: `default`). Each becomes an ordinary feature
    or field of that name, so expressions and steps read it like any other. Built alone the defaults
    stand; built as a part, whatever its master gives takes their place."""
    asks, given = doc.get("given", {}), given or {}
    for name in given.get("names", []):
        if name not in asks:
            diag.error("give.unknown", name, f"the master gives '{name}', and this world declares no `[given.{name}]`",
                       hint="it declares: " + (", ".join(asks) or "nothing"))
    x0, y0, x1, y1 = doc["world"]["extent"]
    for name, ask in asks.items():
        if ask.get("type") == "features":      # a set of one kind: what is given takes the place of this world's own
            if name in given.get("features", {}):
                doc["feature"] = [f for f in doc.get("feature", []) if f.get("kind") != ask.get("kind")] + \
                                 [{**row, "kind": ask.get("kind", row["kind"]), "given": True} for row in given["features"][name]]
        elif ask.get("type") == "region":
            pts = given.get("regions", {}).get(name) or [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
            doc.setdefault("feature", []).insert(0, {"id": name, "kind": ask.get("kind", "guide"), "region": pts, "given": name in given.get("regions", {})})
        else:
            file = given.get("fields", {}).get(name)
            doc.setdefault("field", {})[name] = {"file": file} if file else {"base": ask.get("default", 0.0)}
            doc.setdefault("export", {}).setdefault("fields", [])
            if name not in doc["export"]["fields"]:
                doc["export"]["fields"] = doc["export"]["fields"] + [name]      # so the world's steps are handed it
    if given.get("key"):
        doc["world"]["given"] = given["key"]       # part of every step's record: given something else, they run again


def part_mounts(parts: list[dict]) -> dict:
    """The places a master may name in its parts, in the master's frame: `part.path.start`, `part.path.end`,
    `part.point.at` and `part.instance.mount`. A part that moves takes them with it."""
    out = {ref: ex["at"] for ref, ex in part_exits(parts).items()}
    for part in parts:
        pid, (ox, oy), pw = part["id"], part["offset"], part["world"]
        for fid, shape in pw.shapes.items():
            if shape.path is not None:
                for name, k in (("start", 0), ("end", -1)):
                    out[f"{pid}.{fid}.{name}"] = (float(shape.path.pts[k][0]) + ox, float(shape.path.pts[k][1]) + oy)
            elif "at" in shape.feat:
                out[f"{pid}.{fid}.at"] = (float(shape.feat["at"][0]) + ox, float(shape.feat["at"][1]) + oy)
        for inst in part["instances"]:
            if "id" in inst:
                for name, (mx, my) in pw.instance_mounts(inst["asset"], inst["x"], inst["y"], inst.get("yaw", 0.0), inst.get("scale", 1.0)).items():
                    out[f"{pid}.{inst['id']}.{name}"] = (mx + ox, my + oy)
    return out


def part_exits(parts: list[dict]) -> dict:
    """The exits a master must answer, in the master's frame: `part.<id>` for every point feature of a
    kind flagged `exit`. Each has a place, the way it faces (along the path it stands `on`, else square
    to the nearest side), and the feature's own attributes. The compiler reads one of them, `width`:
    the stretch of edge the exit takes up. The rest it passes on unread."""
    out = {}
    for part in parts:
        pid, (ox, oy), pw = part["id"], part["offset"], part["world"]
        x0, y0, x1, y1 = part["rect"]
        for feat in pw.features:
            if not pw.kinds.get(feat.get("kind", ""), {}).get("exit") or "at" not in feat:
                continue
            x, y = float(feat["at"][0]), float(feat["at"][1])
            side = min(((x - x0, (-1.0, 0.0)), (x1 - x, (1.0, 0.0)), (y - y0, (0.0, -1.0)), (y1 - y, (0.0, 1.0))), key=lambda s: s[0])[1]
            face = side
            host = pw.shapes.get(feat.get("on", {}).get("feature", "")) if isinstance(feat.get("on"), dict) else None
            if host is not None and host.path is not None:
                tan = host.path.at(float(feat["on"].get("t", 0.5)))[1]
                sign = 1.0 if tan[0] * side[0] + tan[1] * side[1] >= 0 else -1.0      # out of the part, whichever way the path was drawn
                face = (float(tan[0]) * sign, float(tan[1]) * sign)
            facts = {k: v for k, v in feat.items() if k not in ("id", "kind", "at", "on") and not k.startswith("_") and isinstance(v, (str, int, float))}
            out[f"{pid}.{feat['id']}"] = {"at": (x + ox, y + oy), "faces": face, "part": pid, "name": feat["id"], "facts": facts}
    return out


def check_exits(world: World, exits: dict) -> None:
    """Every exit a part declares is a promise that something carries on from there. The master
    keeps it by starting or ending a path at the exit; an exit nothing answers is an error."""
    met = {str(f.get(k)) for f in world.features for k in ("starts_at", "ends_at") if k in f}
    for ref, ex in exits.items():
        if ref not in met:
            facts = ", ".join(f"{k} {v}" for k, v in ex["facts"].items())
            world.diag.error("exit.unanswered", ref, f"'{ex['part']}' declares the exit '{ex['name']}'" + (f" ({facts})" if facts else "")
                             + ", and nothing in this world starts or ends there", where=ex["at"],
                             hint=f'carry it on: a path with starts_at = "{ref}"; if it leads nowhere, give that path `onward = <metres>` and a kind that dies out')


def part_paths(parts: list[dict]) -> dict:
    """The lines a master may name in its seam parts, in the master's frame: `part.edge` is the whole
    outline of its footprint, run with the part on the left, and `part.edge.<n>` one side of it.
    A box also offers its sides as south, east, north and west."""
    out = {}
    for part in parts:
        if "portal" in part["spec"]:
            continue
        x0, y0, x1, y1 = part["rect"]
        ring = part.get("poly") or [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
        if sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1])) < 0:
            ring = ring[::-1]
        ring = [[q[0] + part["offset"][0], q[1] + part["offset"][1]] for q in ring]
        out[f"{part['id']}.edge"] = ring + ring[:1]
        for k, side in enumerate(zip(ring, ring[1:] + ring[:1])):
            out[f"{part['id']}.edge.{k}"] = list(side)
            if "poly" not in part:
                out[f"{part['id']}.edge.{('south', 'east', 'north', 'west')[k]}"] = list(side)
    return out


def _blend(spec: dict, w: np.ndarray, d: np.ndarray, diag: Diagnostics) -> np.ndarray:
    """The falloff a part asks for: an expression of `w` (0 at the edge, 1 an apron inside) and `d`
    (metres inside the edge). Without one the weight is used as it is."""
    if "blend" not in spec:
        return w
    env = {"__builtins__": {}, "w": w, "d": d, "apron": float(spec.get("apron", 20.0)), "smoothstep": smoothstep,
           "min": np.minimum, "max": np.maximum, "clamp": np.clip, "where": np.where, "sqrt": np.sqrt}
    try:
        return np.clip(np.asarray(eval(str(spec["blend"]), env), dtype=np.float32) * np.ones_like(w), 0.0, 1.0)
    except Exception as exc:      # reported, and the part keeps the default falloff
        diag.error("part.blend", spec["id"], f"its `blend` could not be worked out: {exc}", hint="it may use w, d, apron, smoothstep, min, max, clamp, where, sqrt")
        return w


def _window(world: World, part: dict):
    """Where the part's footprint lands on the master grid, and a function that takes any raster of
    the part onto those posts. With the same spacing that is a slice; otherwise the part is sampled
    at the master's posts, so a master may be coarser or finer than what it places."""
    g, pg = world.grid, part["world"].grid
    ox, oy = part["offset"]
    x0, y0, x1, y1 = part["rect"]
    i0, i1 = int(round((x0 + ox - g.x0) / g.res)), int(round((x1 + ox - g.x0) / g.res))
    j0, j1 = int(round((y0 + oy - g.y0) / g.res)), int(round((y1 + oy - g.y0) / g.res))
    if i0 < 0 or j0 < 0 or i1 >= g.nx or j1 >= g.ny:
        return None
    mwin = (slice(j0, j1 + 1), slice(i0, i1 + 1))
    if abs(pg.res - g.res) < 1e-9:
        pi0, pj0 = int(round((x0 - pg.x0) / pg.res)), int(round((y0 - pg.y0) / pg.res))
        pwin = (slice(pj0, pj0 + (j1 - j0) + 1), slice(pi0, pi0 + (i1 - i0) + 1))
        return mwin, lambda arr: arr[(Ellipsis,) + pwin]
    px, py = g.X[mwin] - ox, g.Y[mwin] - oy

    def take(arr):
        if arr.ndim == 3:
            return np.stack([pg.sample(a, px, py) for a in arr]).astype(np.float32)
        return pg.sample(arr, px, py).astype(np.float32)
    return mwin, take


def _edge(world: World, part: dict, shape) -> np.ndarray:
    """Posts from the footprint's edge, over the part's window: 0 on the edge and outside it."""
    h, w = shape
    jj, ii = np.indices(shape)
    edge = np.minimum(np.minimum(ii, w - 1 - ii), np.minimum(jj, h - 1 - jj)).astype(np.float32)
    if part.get("poly"):      # an outline inside the box: the nearer of the two edges counts
        res, (x0, y0) = world.grid.res, part["rect"][:2]
        img = Image.new("L", (w, h), 0)
        ImageDraw.Draw(img).polygon([((x - x0) / res, (y - y0) / res) for x, y in part["poly"]], fill=1)
        if part.get("named"):      # cut to a region of the master: the way an exit stands on is still the
            pw = part["world"]     # part's as far as that exit, or the promise would end in mid air
            carry = 1.5 * float(part["spec"].get("apron", 20.0)) + 4.0
            for feat in pw.features:
                host = pw.shapes.get(feat.get("on", {}).get("feature", "")) if isinstance(feat.get("on"), dict) else None
                if pw.kinds.get(feat.get("kind", ""), {}).get("exit") and host is not None and host.path is not None:
                    beyond = LineString(host.path.pts).difference(Polygon(part["poly"]).buffer(0))      # only the stretch from the region out to
                    for run in getattr(beyond, "geoms", [beyond]):                                       # the exit; its other end stays cut
                        if run.geom_type == "LineString" and run.distance(Point(feat["at"])) < 30.0:
                            ImageDraw.Draw(img).line([((float(q[0]) - x0) / res, (float(q[1]) - y0) / res) for q in run.coords],
                                                     fill=1, width=int((float(feat.get("width", 8.0)) + 2 * carry) / res))
            edge = edge * 3.0      # and at the box itself the blend is short: an exit stands just beyond it
        edge = np.minimum(edge, np.maximum(distance_transform_edt(np.asarray(img, dtype=bool)) - 1.0, 0.0)).astype(np.float32)
    return edge


def _weight(world: World, part: dict, shape, apron_cells: float) -> np.ndarray:
    """0 at the footprint's edge, 1 an apron's width inside it."""
    return smoothstep(0.0, max(apron_cells, 1.0), _edge(world, part, shape))


def _lift(world: World, part: dict, ph: np.ndarray, w: np.ndarray, mwin) -> float:
    """How far a seam part is raised to sit on the land it joins: by its edge ring, or at a declared joint."""
    g, spec, pw, height = world.grid, part["spec"], part["world"], world.fields["height"]
    if "lift" in spec:          # said outright: the part's own heights plus this many metres
        return float(spec["lift"])
    if "joins_at" in spec:      # a declared joint: the part meets the land exactly here, in its own coordinates
        jx, jy = spec["joins_at"]
        return float(g.sample(height, [jx + part["offset"][0]], [jy + part["offset"][1]])[0]
                     - pw.grid.sample(pw.field("height"), [jx], [jy])[0])
    ring = (w > 0) & (w < 0.15)
    return float(height[mwin][ring].mean() - ph[ring].mean())


def _skirts(world: World, parts: list[dict]) -> None:
    """`outer` on a seam part: the master's land rises or falls to meet the part's edge over that many
    metres outside its footprint, so the part keeps every post of its own ground. Where two skirts
    overlap each is weighted by how near its part is, so both edges are still met exactly. No skirt
    reaches into another part's footprint."""
    g, height = world.grid, world.fields["height"]
    todo = []
    claimed = np.zeros(height.shape, dtype=bool)
    for part in parts:
        if "portal" in part["spec"]:
            continue
        win = _window(world, part)
        if win is None:
            continue
        claimed[win[0]] = True
        if float(part["spec"].get("outer", 0.0)) > 0:
            ph = win[1](part["world"].field("height"))
            part["dz"] = _lift(world, part, ph, _weight(world, part, ph.shape, float(part["spec"].get("apron", 20.0)) / g.res), win[0])
            todo.append((part, win[0]))
    if not todo:
        return
    num, den, most = (np.zeros(height.shape, dtype=np.float32) for _ in range(3))
    for part, mwin in todo:
        outer, (ox, oy), pg = float(part["spec"]["outer"]), part["offset"], part["world"].grid
        x0, y0, x1, y1 = part["rect"]
        n = int(np.ceil(outer / g.res))
        gwin = (slice(max(0, mwin[0].start - n), min(g.ny, mwin[0].stop + n)), slice(max(0, mwin[1].start - n), min(g.nx, mwin[1].stop + n)))
        px, py = g.X[gwin] - ox, g.Y[gwin] - oy
        cx, cy = np.clip(px, x0, x1), np.clip(py, y0, y1)
        d = np.hypot(px - cx, py - cy)
        # Carried outward, every wrinkle of the edge would become a rib. So the further out, the more
        # the skirt follows a smoothed copy of the part's ground; at the edge itself it is the real one.
        ph = part["world"].field("height")
        soft = gaussian_filter(ph, sigma=14.0 / pg.res, mode="nearest")
        far = smoothstep(0.0, 24.0, d)
        edge = (pg.sample(ph, cx, cy) * (1 - far) + pg.sample(soft, cx, cy) * far).astype(np.float32) + part["dz"]
        wo = np.where(d > 0, 1.0 - smoothstep(0.0, outer, d), 0.0).astype(np.float32)
        k = wo / (d + 0.25)
        num[gwin] += edge * k
        den[gwin] += k
        most[gwin] = np.maximum(most[gwin], wo)
    most[claimed] = 0.0
    on = den > 0
    height[on] += (num[on] / den[on] - height[on]) * most[on]


def apply_height(world: World, parts: list[dict]) -> None:
    g, diag = world.grid, world.diag
    height = world.field("height")
    _skirts(world, parts)
    for part in parts:
        spec, pw = part["spec"], part["world"]
        win = _window(world, part)
        if win is None:
            diag.error("part.outside", part["id"], "its footprint does not fit inside the master's extent")
            part["skip"] = True
            continue
        mwin, take = win
        if "portal" in spec:        # nothing is blended: the door meets the ground, and that is all
            at = spec["portal"]["at"]
            ground = float(g.sample(height, [at[0]], [at[1]])[0])
            part["dz"] = ground - part["door_z"]
            under = height[mwin]
            relief = float(under.max() - under.min())
            if relief > float(spec.get("max_relief", 0.4)):
                diag.warn("part.ground", part["id"],
                          f"the ground under its footprint varies by {relief:.1f} m and an interior blends with nothing",
                          where=at, hint="level the ground there with a pad, or move the door")
            continue
        ph = take(pw.field("height"))
        w = _weight(world, part, ph.shape, float(spec.get("apron", 20.0)) / g.res)
        ring = (w > 0) & (w < 0.15)
        skirted = float(spec.get("outer", 0.0)) > 0      # the land outside has already come to meet it
        dz = part["dz"] if skirted else _lift(world, part, ph, w, mwin)       # lift the part to sit on the land it joins
        part["dz"] = dz
        step = np.abs(height[mwin] - (ph + dz))[ring]
        limit = float(spec.get("max_step", 3.0))
        if step.max() > limit and not skirted:
            jj, ii = np.nonzero(ring)
            k = int(step.argmax())
            where = (g.x0 + (mwin[1].start + ii[k]) * g.res, g.y0 + (mwin[0].start + jj[k]) * g.res)
            diag.error("part.seam", part["id"],
                       f"at its edge it stands up to {step.max():.1f} m from the land it joins (mean {step.mean():.1f} m); "
                       f"its limit is {limit:.1f} m", where=where,
                       hint="place it on ground shaped like its own, widen its apron, or raise the limit on purpose")
        water = world.doc.get("field", {}).get("water")
        if water is not None and spec.get("expects", "land") == "land":
            dry = ring.copy()               # an exit is where the part says it is crossed: water there was promised
            jj, ii = np.nonzero(ring)
            ex, ey = g.x0 + (mwin[1].start + ii) * g.res, g.y0 + (mwin[0].start + jj) * g.res
            for exit_ in part_exits([part]).values():
                near = np.hypot(ex - exit_["at"][0], ey - exit_["at"][1]) < float(exit_["facts"].get("width", 0.0)) / 2 + 3 * float(spec.get("apron", 20.0))
                dry[jj[near], ii[near]] = False
            wet = float((world.field("water")[mwin][dry] > height[mwin][dry]).mean()) if dry.any() else 0.0
            if wet > 0.02:
                diag.error("part.context", part["id"], f"it expects land all round, and {wet * 100:.0f}% of its edge is under water")
        w = _blend(spec, w, _edge(world, part, ph.shape) * g.res, diag)     # the checks above read the edge ring; the blend itself is the part's to choose
        if skirted and not part.get("poly"):
            height[mwin] = ph + dz
        else:
            height[mwin] = height[mwin] + (ph + dz - height[mwin]) * w
        part["w"], part["mwin"], part["take"] = w, mwin, take
        world.part_w[part["id"]] = np.zeros(height.shape, dtype=np.float32)      # what `inside()` and `outside()` read
        world.part_w[part["id"]][mwin] = w


def apply_late(world: World, parts: list[dict], names: list[str]) -> None:
    """Master features that say `after = "parts"` (or whose layer does) are laid on the joined ground,
    so a ridge along a join or a path through it is not blended away by the parts it crosses. Where
    one of them moves ground a part anchored, that is said: the master wins, but not silently."""
    g = world.grid
    for name in names:
        for feat, kind, eff in world.late(name) if name in world.fields else []:
            before = world.fields[name].copy() if name == "height" else None
            world._apply(name, world.fields[name], feat, kind, eff)
            if before is None:
                continue
            moved = np.abs(world.fields[name] - before)
            for part in parts:
                pw, (ox, oy) = part["world"], part["offset"]
                for pf in pw.features:
                    shp = pw.shapes.get(pf.get("id"))
                    if shp is None or pf.get("authority", pw.kinds.get(pf.get("kind", ""), {}).get("authority", "shaped")) != "anchored":
                        continue
                    pts = np.asarray(shp.polygon if shp.polygon is not None else shp.path.pts[::40] if shp.path is not None else [shp.point], dtype=float)
                    worst = float(g.sample(moved, pts[:, 0] + ox, pts[:, 1] + oy).max())
                    if worst > 0.3:
                        k = int(g.sample(moved, pts[:, 0] + ox, pts[:, 1] + oy).argmax())
                        world.diag.warn("part.overruled", feat["id"],
                                        f"it moves the ground by {worst:.1f} m where '{part['id']}' anchored it for '{pf['id']}'",
                                        where=(pts[k, 0] + ox, pts[k, 1] + oy), hint="move it clear of that, or accept that the master's word is last here")


def check_named(world: World, parts: list[dict]) -> None:
    """A place the master names in a part must be the part's own ground. In the apron the ground is
    a blend, so a road end there no longer stands where its part put it."""
    for feat in world.features:
        for key in ("starts_at", "ends_at"):
            ref = str(feat.get(key, ""))
            part = next((p for p in parts if ref.startswith(p["id"] + ".") and "w" in p), None)
            if part is None or ref not in world.mounts:
                continue
            x, y = world.mounts[ref]
            g, mwin = world.grid, part["mwin"]
            j, i = int(round((y - g.y0) / g.res)) - mwin[0].start, int(round((x - g.x0) / g.res)) - mwin[1].start
            if 0 <= j < part["w"].shape[0] and 0 <= i < part["w"].shape[1] and part["w"][j, i] < 0.98:
                world.diag.warn("part.named_in_apron", feat["id"],
                                f"`{key}` names '{ref}', which lies in the apron of '{part['id']}': the ground there is "
                                f"{(1 - part['w'][j, i]) * 100:.0f}% the master's, not the part's", where=(x, y),
                                hint="narrow the part's apron or change its `blend`, or move that place further inside the part")


def apply_fields(world: World, parts: list[dict]) -> None:
    """After the master's own fields exist: inside a seam part's footprint they become the part's."""
    for part in parts:
        if part.get("skip") or "w" not in part:
            continue
        pw, w, mwin, take = part["world"], part["w"], part["mwin"], part["take"]
        # a field measured in metres is lifted with the part, or its lakes would stay behind
        # (a part that expects sea shares the master's sea level, so its water is left where it is)
        lift = {n: part.get("dz", 0.0) for n, f in part["doc"].get("field", {}).items()
                if f.get("unit") == "m" and part["spec"].get("expects", "land") != "sea"}
        for name, arr in list(world.fields.items()):
            if name in ("height", "bed", "hole") or arr.ndim != 2:
                continue
            if name in pw.fields and pw.fields[name].ndim == 2:
                arr[mwin] = arr[mwin] + (take(pw.fields[name]) + lift.get(name, 0.0) - arr[mwin]) * w
            elif name in world.doc.get("field", {}) and "vegetation" in world.doc["field"][name].get("tags", []):
                arr[mwin] = arr[mwin] * (1 - w)          # the master grows nothing of its own inside a part
        _carry_under(world, part)
        ground = world.fields["ground"]
        named = part["spec"].get("ground_as", {})        # a part's material may be carried as one the master has
        for mat in pw.materials:                          # the part may bring materials the master lacks
            if named.get(mat, mat) not in world.materials:
                world.materials.append(named.get(mat, mat))
                ground = np.concatenate([ground, np.zeros((1,) + ground.shape[1:], dtype=np.float32)])
        world.fields["ground"] = ground
        ground[(slice(None),) + mwin] *= (1 - w)[None]
        pground = take(pw.fields["ground"])
        for k, mat in enumerate(pw.materials):
            ground[(world.materials.index(named.get(mat, mat)),) + mwin] += pground[k] * w


def _carry_under(world: World, part: dict) -> None:
    """A seam part with volumes has ground that is more than a surface: `hole` says where the meshed
    volume replaces the heightfield, and `bed` is the lowest floor under it. Both come with the part,
    or its mesh would arrive under ground that was never cut open.

    The mesh is brought whole and rigid, so the hole is copied as it is. The bed keeps the part's
    clearance under its top, scaled by the same weight the height was blended with. In the apron the
    ground is a blend and cannot meet a rigid mesh, so a volume that reaches into it is reported."""
    pw, w, mwin, take = part["world"], part["w"], part["mwin"], part["take"]
    if "hole" not in pw.fields or "bed" not in pw.fields:
        return
    g, height = world.grid, world.fields["height"]
    cut = take(pw.fields["hole"]) > 0.5
    if not cut.any():                 # it declares a volume field and hosts nothing in this footprint
        return
    hole = world.fields.setdefault("hole", np.zeros((g.ny, g.nx), dtype=np.float32))
    bed = world.fields.setdefault("bed", height.copy())
    hole[mwin] = np.maximum(hole[mwin], take(pw.fields["hole"]))
    room = np.maximum(take(pw.fields["height"]) - take(pw.fields["bed"]), 0.0) * w
    bed[mwin] = np.minimum(bed[mwin], height[mwin] - room)
    loose = cut & (w < 0.999)
    if loose.any():
        jj, ii = np.nonzero(loose)
        k = int(np.argmin(w[jj, ii]))
        where = (g.x0 + (mwin[1].start + ii[k]) * g.res, g.y0 + (mwin[0].start + jj[k]) * g.res)
        world.diag.warn("part.volume_apron", part["id"],
                        f"{float(loose.sum()) * g.res * g.res:.0f} m2 of its volumes lie in its apron, where the ground is blended "
                        "and does not meet their mesh", where=where,
                        hint="narrow the apron, or grow the footprint so the volumes stand clear of its edge")


def bring(world: World, parts: list[dict], built: dict, volumes: list[dict]) -> list[dict]:
    """Everything the parts place, in the master's frame and under the part's name. Instances are
    returned; features go into `world`, generated meshes into `built` and volume meshes into `volumes`."""
    out = []
    for part in parts:
        if part.get("skip"):
            continue
        pid, off, dz = part["id"], part["offset"], part.get("dz", 0.0)
        for inst in part["instances"]:
            new = dict(inst)
            new["x"], new["y"] = inst["x"] + off[0], inst["y"] + off[1]
            if part.get("poly") and float(world.grid.sample(world.part_w[pid], [new["x"]], [new["y"]])[0]) <= 0:
                continue                         # it stands outside the outline the master gave this part
            new["poly"] = [[p[0] + off[0], p[1] + off[1]] for p in inst["poly"]] if "poly" in inst else None
            if "id" in inst:
                new["id"] = f"{pid}.{inst['id']}"
            if "in" in new and new["in"]:
                new["in"] = f"{pid}.{new['in']}"
            if "floor_z" in inst:
                new["floor_z"] = inst["floor_z"] + dz
            new["part"] = pid
            out.append(new)
        for feat in part["world"].features:
            if feat.get("id") not in part["world"].shapes or feat.get("of_instance"):
                continue
            new = {k: copy.copy(v) for k, v in feat.items() if k not in ("on", "flood", "_mask", "path3")}
            new["id"] = f"{pid}.{feat['id']}"
            if new.get("in"):                   # the room it is in is brought under the part's name too
                new["in"] = f"{pid}.{new['in']}"
            for key in ("path", "region"):
                if key in new:
                    new[key] = [[p[0] + off[0], p[1] + off[1]] for p in new[key]]
            if "at" in new:
                new["at"] = [new["at"][0] + off[0], new["at"][1] + off[1]]
            if "ellipse" in new:
                c = new["ellipse"]["center"]
                new["ellipse"] = {"center": [c[0] + off[0], c[1] + off[1]], "size": new["ellipse"]["size"]}
            if isinstance(new.get("look"), list):
                new["look"] = [new["look"][0] + off[0], new["look"][1] + off[1]]
            elif isinstance(new.get("look"), dict):
                # A place on one of the part's paths. That path is not the master's to measure along,
                # so the place is worked out here, in the part's frame, and brought as a point.
                look = new.pop("look")
                host = part["world"].shapes.get(look.get("feature", ""))
                if host is None or host.path is None:      # reported and left out, as a master's own would be
                    world.diag.error("frame.unknown", new["id"], f"`look` names '{look.get('feature')}', which is not a path")
                    continue
                p, _, _ = host.path.at(float(look.get("t", 0.5)))
                new["look"] = [float(p[0]) + off[0], float(p[1]) + off[1]]
            if "floor_z" in new:
                new["floor_z"] = new["floor_z"] + dz
            if "cut_z" in new:
                new["cut_z"] = new["cut_z"] + dz
            new["part"] = pid
            world.add_feature(new)
        for ex in part["doc"].get("expect", []):
            world.doc.setdefault("expect", []).append(
                {**ex, "from": f"{pid}.{ex['from']}", "to": [f"{pid}.{t}" for t in ex.get("to", [])], "why": f"{pid}: {ex.get('why', '')}"})
        for name, prof in part["doc"].get("profile", {}).items():
            world.doc.setdefault("profile", {}).setdefault(name, prof)
        shift = np.array([off[0], off[1], dz])
        for m in part["built"].get("meshes", []):
            built.setdefault("meshes", []).append({**m, "id": f"{pid}_{m['id']}", "z0": m["z0"] + dz,
                                                   "parts": [(v + shift, n, f) for v, n, f in m["parts"]]})
        for vol in part["volumes"]:      # a volume's mesh is in world coordinates already, so it moves as a whole
            volumes.append({**vol, "id": f"{pid}_{vol['id']}", "verts": (vol["verts"] + shift).astype("<f4")})
        for light in part["built"].get("lights", []):
            built.setdefault("lights", []).append({**light, "pos": [light["pos"][0] + off[0], light["pos"][1] + off[1], light["pos"][2] + dz]})
        for rid, room in part["built"].get("rooms", {}).items():
            built.setdefault("rooms", {})[f"{pid}.{rid}"] = room
        if "portal" in part["spec"]:      # an interior's floors replace the ground under them
            g = world.grid
            img = Image.new("L", (g.nx, g.ny), 0)
            for rid in part["built"].get("rooms", {}):
                poly = part["world"].shapes[rid].polygon
                ImageDraw.Draw(img).polygon([((p[0] + off[0] - g.x0) / g.res, (p[1] + off[1] - g.y0) / g.res) for p in poly], fill=1)
            hole = world.fields.setdefault("hole", np.zeros((g.ny, g.nx), dtype=np.float32))
            world.fields["hole"] = np.maximum(hole, np.asarray(img, dtype=np.float32))
            world.fields.setdefault("bed", world.fields["height"].copy())
        n_inst = sum(1 for i in out if i.get("part") == pid)
        kind = "portal" if "portal" in part["spec"] else "seam"
        print(f"  part {pid} ({kind}): {n_inst} placed, lifted {dz:+.1f} m, moved by ({off[0]:g}, {off[1]:g})")
    return out
