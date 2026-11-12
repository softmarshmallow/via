"""Producers: everything that lowers something compact into plain instance specs or instances.

Owns `expand` (sets, runs and kit runs become instance specs), `dress` (small things strewn on a
placed instance, and the flank pairs either side of a door) and `scatter` (a density field becomes
instances). Nothing downstream knows a producer ran: what comes out is what a hand would have written.
"""
from __future__ import annotations

import json
import math

import numpy as np
from shapely import contains_xy
from shapely.geometry import Polygon
from PIL import Image, ImageDraw
from scipy.ndimage import binary_dilation
from scipy.spatial import cKDTree

from .geometry import rect_polygon
from .placement import place_explicit, resolve_position
from .world import World


def select(world: World, choices: list[dict]) -> list[dict]:
    """A choice whose id starts with `@` names a place in the pack's taxonomy, not an asset: it
    stands for every asset filed at or under it, so a world can ask for a role and let the pack answer."""
    out = []
    for c in choices:
        if not str(c.get("id", "")).startswith("@"):
            out.append(c)
            continue
        want = c["id"][1:]
        hits = sorted(a for a, v in world.pack["asset"].items() if v.get("taxon") == want or v.get("taxon", "").startswith(want + "/"))
        if not hits:
            world.diag.error("asset.unknown", c["id"], f"the pack files no asset under '{want}'")
        out += [{**c, "id": a} for a in hits]
    return out


def expand(world: World) -> list[dict]:
    """Sets and runs lower to plain instance specs before anything is placed.
    A set is a group of items in one local frame, placed as a unit. A run repeats one asset along a
    stretch of a feature's outline. Call it once per world: a kit run also adds its pad as a feature."""
    out: list[dict] = []
    sets = world.doc.get("set", {})
    # a kit run: bays of one module laid end to end between two points, on one shared pad
    for n, kr in enumerate(world.doc.get("kitrun", [])):
        kid = kr.get("id", f"kitrun_{n}")
        # `bay` is one asset, or a list of weighted choices that share one module. `end` closes both ends.
        choices = kr.get("bay", "")
        choices = [{"id": choices}] if isinstance(choices, str) else list(choices)
        choices = select(world, choices)
        names = [c["id"] for c in choices] + ([kr["end"]] if "end" in kr else [])
        missing = [a for a in names if a not in world.pack["asset"]]
        if missing or not choices:
            world.diag.error("asset.unknown", kid, f"the pack has no asset '{', '.join(missing) or kr.get('bay')}'")
            continue
        bay = world.pack["asset"][choices[0]["id"]]
        modules = {a: float(world.pack["asset"][a].get("module_m") or world.pack["asset"][a]["footprint"][0]) for a in names}
        if len(set(modules.values())) > 1:
            world.diag.error("kit.module", kid, "the pieces of one run must share a module: "
                             + ", ".join(f"{a} {m:g} m" for a, m in modules.items()), hint="pieces of one kit are made to one module")
            continue
        if "along" in kr:      # frontage: the run stands beside a street and follows it when the street moves
            host = world.shapes.get(kr["along"])
            if host is None or host.path is None:
                world.diag.error("frame.unknown", kid, f"`along` names '{kr['along']}', which is not a path")
                continue
            left = kr.get("side", "left") == "left"
            run_m = int(kr.get("bays", 1)) * modules[choices[0]["id"]]
            t0 = float(kr.get("at", 0.0))
            a = host.path.at(t0)
            b = host.path.at(min(1.0, t0 + run_m / host.path.length))
            chord = np.asarray(b[0], float) - np.asarray(a[0], float)
            chord = chord / max(float(np.linalg.norm(chord)), 1e-9)
            normal = np.array([-chord[1], chord[0]]) * (1.0 if left else -1.0)
            back = float(kr.get("setback", 0.0)) + bay["footprint"][1] / 2
            start = np.asarray(a[0], float) + normal * back
            p0, p1 = start, start + chord * run_m
            if left:      # bays face 90 degrees left of the run, so a run on the street's left is laid backwards
                p0, p1 = p1, p0
            kr = {**kr, "faces": "left"}
        else:
            p0, p1 = np.asarray(kr["from"], dtype=float), np.asarray(kr["to"], dtype=float)
        length = float(np.linalg.norm(p1 - p0))
        d = (p1 - p0) / max(length, 1e-9)
        module = float(bay.get("module_m") or bay["footprint"][0])
        count = max(1, int(round(length / module)))
        slack = length - count * module
        mid = (p0 + p1) / 2
        if abs(slack) > max(float(kr.get("tolerance", 0.01)) * length, 0.08):
            other = (count + (1 if slack > 0 else -1)) * module
            world.diag.error("kit.misfit", kid,
                             f"a run of {length:.1f} m takes {count} bays of {module:g} m and leaves {slack:+.1f} m; "
                             f"{count * module:g} m or {other:g} m would close", where=mid,
                             hint="change the run's length to a whole number of bays, or add a filler piece")
        ang = math.degrees(math.atan2(d[1], d[0]))
        yaw = ang + (90.0 if kr.get("faces", "left") == "left" else -90.0)
        pick = np.random.default_rng([world.seed, int(kr.get("seed", n)), 7919])
        weights = np.array([float(c.get("weight", 1.0)) for c in choices])
        for k in range(count):
            c = p0 + d * ((k + 0.5) * length / count)
            asset = kr["end"] if "end" in kr and k in (0, count - 1) else choices[int(pick.choice(len(choices), p=weights / weights.sum()))]["id"]
            out.append({"id": f"{kid}.{k}", "asset": asset, "at": [float(c[0]), float(c[1])], "yaw": yaw, "group": kr.get("group", kid), "_layer": kr.get("_layer")})
        margin = float(kr.get("margin", 1.0))
        world.add_feature({"id": kid, "kind": kr.get("kind", "pad"), "of_instance": True,
                           "region": rect_polygon(mid, [length + 2 * margin, bay["footprint"][1] + 2 * margin], yaw)})
    for n, run in enumerate(world.doc.get("run", [])):
        count = int(run.get("count", 1))
        t0, t1 = float(run.get("from", 0.0)), float(run.get("to", 1.0))
        for k in range(count):
            child = {key: val for key, val in run.items() if key not in ("from", "to", "count", "on")}
            child["id"] = f"{run.get('id', f'run_{n}')}.{k}"
            child.setdefault("group", run.get("id", f"run_{n}"))      # neighbours in a run may meet
            child["on"] = {"feature": run["on"], "t": (t0 + (t1 - t0) * (k + 0.5) / count) % 1.0}
            if "offset" in run:             # beside a path, not against a wall
                child["on"]["offset"] = run["offset"]
                child.pop("offset", None)
                child.setdefault("face", "along")
            else:
                child.setdefault("against", True)
            out.append(child)
    for spec in world.doc.get("instance", []):
        if "set" not in spec:
            out.append(spec)
            continue
        sdef = sets.get(spec["set"])
        if sdef is None:
            world.diag.error("set.unknown", spec.get("id", "?"), f"no set named '{spec['set']}'")
            continue
        pos = resolve_position(world, spec, spec.get("id", "?"))
        if pos is None:
            continue
        x, y, yaw = pos
        th = math.radians(yaw)
        for item in sdef.get("items", []):
            child = {"id": f"{spec['id']}.{item['name']}", "asset": item["asset"], "in": spec.get("in"), "_layer": spec.get("_layer")}
            dx, dy = item.get("offset", [0.0, 0.0])
            if "atop" in item:
                child.update({"atop": f"{spec['id']}.{item['atop']}", "offset": [dx, dy], "turn": item.get("turn", 0.0)})
            else:
                child.update({"at": [x + math.sin(th) * dx + math.cos(th) * dy, y - math.cos(th) * dx + math.sin(th) * dy],
                              "yaw": yaw + item.get("turn", 0.0)})
            out.append(child)
    return out


def dress(world: World, explicit: list[dict]) -> list[dict]:
    """Small things strewn on the top of something already placed, seeded."""
    specs = []
    for n, d in enumerate(world.doc.get("dress", [])):
        host = next((i for i in explicit if i["id"] == d.get("atop")), None)
        if host is None:
            world.diag.error("dress.no_host", d.get("id", f"dress_{n}"), f"'{d.get('atop')}' is not a placed instance")
            continue
        rng = np.random.default_rng(world.seed * 101 + int(d.get("seed", n)))
        half = [max(0.0, host["footprint"][0] / 2 - float(d.get("margin", 0.08))), max(0.0, host["footprint"][1] / 2 - float(d.get("margin", 0.08)))]
        weights = np.array([float(c.get("weight", 1.0)) for c in d["assets"]])
        placed: list = []
        for k in range(int(d.get("count", 3))):
            for _ in range(30):
                at = [float(rng.uniform(-half[0], half[0])), float(rng.uniform(-half[1], half[1]))]
                if all(math.hypot(at[0] - q[0], at[1] - q[1]) > float(d.get("gap", 0.16)) for q in placed):
                    placed.append(at)
                    choice = d["assets"][int(rng.choice(len(weights), p=weights / weights.sum()))]
                    specs.append({"id": f"{d.get('id', f'dress_{n}')}.{k}", "asset": choice["id"], "atop": host["id"], "_layer": d.get("_layer"),
                                  "offset": at, "turn": float(rng.uniform(0, 360))})
                    break
    for n, fl in enumerate(world.doc.get("flank", [])):      # a pair of things either side of each front door
        for hid in fl.get("hosts", []):
            host = next((i for i in explicit if i["id"] == hid), None)
            if host is None:
                world.diag.error("flank.no_host", fl.get("id", f"flank_{n}"), f"'{hid}' is not a placed instance")
                continue
            forward = host["footprint"][1] / 2 + float(fl.get("forward", 0.9))
            for side, sign in (("l", -1.0), ("r", 1.0)):
                specs.append({"id": f"{fl.get('id', f'flank_{n}')}.{hid}.{side}", "asset": fl["asset"], "beside": hid, "_layer": fl.get("_layer"),
                              "offset": [sign * float(fl.get("spread", 1.8)), forward]})
    return place_explicit(world, specs) if specs else []


def occupied_mask(world: World, instances: list[dict], grow_m: float = 1.0) -> np.ndarray:
    g = world.grid
    img = Image.new("L", (g.nx, g.ny), 0)
    draw = ImageDraw.Draw(img)
    for inst in instances:
        if world.pack["asset"][inst["asset"]]["solid"]:
            draw.polygon([((x - g.x0) / g.res, (y - g.y0) / g.res) for x, y in inst["poly"]], fill=1)
    mask = np.asarray(img, dtype=bool)
    return binary_dilation(mask, iterations=max(1, int(grow_m / g.res)))


def scatter(world: World, explicit: list[dict]) -> list[dict]:
    """Producers. Each one lowers a density field to plain instances; nothing downstream knows it ran."""
    g = world.grid
    occupied = occupied_mask(world, explicit).astype(np.float32)
    out = []
    for n, spec in enumerate(world.doc.get("scatter", [])):
        sid = spec.get("id", f"scatter_{n}")
        choices = [c for c in spec.get("assets", []) if c["id"] in world.pack["asset"]]
        for c in spec.get("assets", []):
            if c["id"] not in world.pack["asset"]:
                world.diag.error("asset.unknown", sid, f"the pack has no asset '{c['id']}'")
        if not choices:
            continue
        env = world.env()

        def around(other: str, reach: float, _out=out):
            """1 beside what an earlier scatter placed, falling to 0 at `reach` metres: growth by cause."""
            pts = np.array([[i["x"], i["y"]] for i in _out if i.get("by") == other], dtype=float)
            if not len(pts):
                return np.zeros((g.ny, g.nx), np.float32)
            dist = cKDTree(pts).query(np.column_stack([g.X.ravel(), g.Y.ravel()]))[0].reshape(g.ny, g.nx)
            return (1.0 - np.clip(dist / float(reach), 0, 1)).astype(np.float32)

        env["around"] = around
        density = np.clip(world.evaluate(spec.get("density", 1.0), env, (g.ny, g.nx), f"scatter.{sid}"), 0, 1)
        rng = np.random.default_rng(world.seed * 7919 + int(spec.get("seed", n)))
        cell = float(spec["spacing"])
        nx, ny = int((g.x1 - g.x0) / cell), int((g.y1 - g.y0) / cell)
        jx = rng.uniform(0.1, 0.9, (ny, nx))
        jy = rng.uniform(0.1, 0.9, (ny, nx))
        xs = (g.x0 + (np.arange(nx)[None, :] + jx) * cell).ravel()
        ys = (g.y0 + (np.arange(ny)[:, None] + jy) * cell).ravel()
        keep = rng.uniform(0, 1, xs.shape) < g.sample(density, xs, ys)
        keep &= g.sample(occupied, xs, ys) < 0.5
        weights = np.array([float(c.get("weight", 1.0)) for c in choices])
        pick = rng.choice(len(choices), size=xs.shape, p=weights / weights.sum())
        lo, hi = spec.get("scale", [1.0, 1.0])
        scales = rng.uniform(lo, hi, xs.shape)
        yaws = rng.uniform(0, 360, xs.shape)
        tint_lo, tint_hi = spec.get("tint", [0.86, 1.08])
        tints = rng.uniform(tint_lo, tint_hi, (xs.shape[0], 3)) * rng.uniform(0.94, 1.06, (xs.shape[0], 1))
        # hand decisions about places, which hold whatever the rule later does: a `clear` keeps the
        # rule out of a spot, and a `pin` stands a chosen thing there (and keeps the rule off it)
        for row in world.doc.get("clear", []) + world.doc.get("pin", []):
            if row.get("of") not in (None, sid) and sid not in (row.get("of") if isinstance(row.get("of"), list) else []):
                continue
            radius = float(row.get("radius", row.get("clears", 0.6 * cell)))
            keep &= np.hypot(xs - float(row["at"][0]), ys - float(row["at"][1])) > radius
        locked = np.zeros(xs.shape, bool)
        for lock in world.doc.get("lock", []):      # inside a lock the rule is not asked; a record is replayed
            if lock.get("of") not in (None, sid) and sid not in (lock.get("of") if isinstance(lock.get("of"), list) else []):
                continue
            inside = contains_xy(Polygon(lock["region"]), xs, ys)
            record = world.lock_dir / f"{lock['id']}.{sid}.json" if world.lock_dir else None
            wanted = {int(k) for k in np.nonzero(keep & inside)[0]}
            if record is not None and record.is_file():
                held = json.loads(record.read_text())
                had = {int(r["cell"]) for r in held}
                if had != wanted:
                    world.diag.info("lock.holds", lock["id"], f"'{sid}' would now add {len(wanted - had)} and drop {len(had - wanted)} here; the lock keeps what was recorded")
                for r in held:
                    out.append({k: v for k, v in r.items() if k != "cell"} | {"by": sid, "locked": lock["id"]})
            elif record is not None:
                record.parent.mkdir(parents=True, exist_ok=True)
                rows = [{"cell": int(k), "id": f"{sid}:{int(k)}", "asset": choices[int(pick[k])]["id"], "x": float(xs[k]), "y": float(ys[k]),
                         "yaw": float(yaws[k]), "scale": float(scales[k]), "tint": [float(t) for t in tints[k]]} for k in sorted(wanted)]
                record.write_text(json.dumps(rows, indent=1))
                world.diag.info("lock.taken", lock["id"], f"recorded {len(rows)} of '{sid}'")
                for r in rows:
                    out.append({k: v for k, v in r.items() if k != "cell"} | {"by": sid, "locked": lock["id"]})
            locked |= inside
        keep &= ~locked
        for ci, choice in enumerate(choices):
            asset = world.pack["asset"][choice["id"]]
            sel = keep & (pick == ci)
            for name in asset["avoid"]:
                if name in world.doc.get("field", {}):
                    sel &= g.sample(world.field(name), xs, ys) < 0.5
            for k in np.nonzero(sel)[0]:
                # its name is the rule and the grid cell it came from: the same while spacing, seed and extent hold
                out.append({"id": f"{sid}:{int(k)}", "asset": choice["id"], "x": float(xs[k]), "y": float(ys[k]), "yaw": float(yaws[k]),
                            "scale": float(scales[k]), "tint": [float(t) for t in tints[k]], "by": sid})
                clump = spec.get("clump")
                if clump:      # the kept one is the middle of a group: a few more gather round it, smaller outward
                    crng = np.random.default_rng([world.seed, int(spec.get("seed", n)), int(k)])
                    lo_n, hi_n = clump.get("size", [2, 4])
                    for m in range(int(crng.integers(lo_n, hi_n + 1))):
                        ang, far = crng.uniform(0, 2 * math.pi), float(clump.get("radius", 4.0)) * math.sqrt(crng.uniform(0.08, 1.0))
                        cx, cy = float(xs[k]) + far * math.cos(ang), float(ys[k]) + far * math.sin(ang)
                        if not g.inside(cx, cy) or float(g.sample(occupied, [cx], [cy])[0]) >= 0.5:
                            continue
                        if any(nm in world.doc.get("field", {}) and float(g.sample(world.field(nm), [cx], [cy])[0]) >= 0.5 for nm in asset["avoid"]):
                            continue
                        shrink = 1.0 - float(clump.get("shrink", 0.35)) * far / float(clump.get("radius", 4.0))
                        out.append({"id": f"{sid}:{int(k)}.{m}", "asset": choice["id"], "x": cx, "y": cy, "yaw": float(crng.uniform(0, 360)),
                                    "scale": float(scales[k]) * shrink * float(crng.uniform(0.85, 1.05)), "tint": [float(t) for t in tints[k]], "by": sid})
        world.diag.info("scatter.count", sid, f"{sum(1 for i in out if i.get('by') == sid)} instances")
    # hand decisions about a named result: these hold only while the name does
    by_id = {i["id"]: i for i in out if "id" in i}
    for row in world.doc.get("override", []):
        target = by_id.get(row["id"])
        if target is None:
            world.diag.warn("override.orphan", row["id"], "the thing this override names is no longer produced; the override does nothing",
                            hint="its rule's spacing, seed or extent changed, or the rule no longer keeps it; state the decision as a `pin` or a `clear` at a place")
            continue
        if row.get("remove"):
            out.remove(target)
            continue
        if "at" in row:
            target["x"], target["y"] = float(row["at"][0]), float(row["at"][1])
        for key in ("asset", "yaw", "scale"):
            if key in row:
                target[key] = row[key]
        target["edited"] = True
    for n, row in enumerate(world.doc.get("pin", [])):
        if row["asset"] not in world.pack["asset"]:
            world.diag.error("asset.unknown", row.get("id", f"pin_{n}"), f"the pack has no asset '{row['asset']}'")
            continue
        x, y = float(row["at"][0]), float(row["at"][1])
        for name in world.pack["asset"][row["asset"]]["avoid"]:
            if name in world.doc.get("field", {}) and float(g.sample(world.field(name), [x], [y])[0]) > 0.5:
                world.diag.warn("pin.conflict", row.get("id", f"pin_{n}"), f"it is pinned where '{name}' now says to keep clear",
                                where=(x, y), hint="something moved onto it; move the pin or remove it")
        out.append({"id": row.get("id", f"pin_{n}"), "asset": row["asset"], "x": x, "y": y, "yaw": float(row.get("yaw", 0.0)),
                    "scale": float(row.get("scale", 1.0)), "tint": [1.0, 1.0, 1.0], "by": row.get("of", "pin"), "edited": True})
    return out
