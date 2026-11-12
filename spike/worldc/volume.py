"""Volumes: where one height per point is not enough.

A volume field is a field over x, y and z, but it exists only inside the box of a feature whose kind
hosts it. Positive is solid. Its base is the ground itself (`height - z`), so an untouched volume is
exactly the heightfield; effects then carve or add. At the box's sides the result is blended back to
the ground, so the meshed surface meets the terrain without a gap: that blend is the volume's apron.

After meshing, the top of the solid is written back into `height`, so everything that stands on the
ground keeps working, and the lowest floor is kept as `bed` for water and for walking underneath.
"""
from __future__ import annotations

import numpy as np
from scipy.ndimage import gaussian_filter, map_coordinates, maximum_filter
from scipy.spatial import cKDTree
from skimage.measure import marching_cubes

from .geometry import _perlin, smoothstep
from .world import Unresolved, World


def _fbm3(x, y, z, seed: int, wavelength: float, octaves: int) -> np.ndarray:
    """Three planes of 2D noise, rotated, summed: cheap, and enough to roughen a rock face."""
    total, amp, norm, freq = 0.0, 1.0, 0.0, 1.0 / wavelength
    for o in range(int(octaves)):
        s = seed * 131 + o * 17
        total = total + amp * (_perlin(x * freq + 0.37 * z * freq, y * freq - 0.29 * z * freq, s)
                               + _perlin(y * freq + 11.3, z * freq * 1.3 + 0.41 * x * freq, s + 5)
                               + _perlin(z * freq * 1.3 - 7.1, x * freq + 0.33 * y * freq, s + 9)) / 3.0
        norm += amp
        amp *= 0.5
        freq *= 2.0
    return total / norm


def _dense3(points3: list) -> np.ndarray:
    p = np.asarray(points3, dtype=np.float64)
    seg = np.linalg.norm(np.diff(p, axis=0), axis=1)
    s = np.concatenate([[0.0], np.cumsum(seg)])
    si = np.linspace(0.0, s[-1], max(2, int(s[-1] / 0.4)))
    dense = np.stack([np.interp(si, s, p[:, k]) for k in range(3)], axis=1)
    for _ in range(6):                                      # round the corners
        dense[1:-1] = 0.25 * dense[:-2] + 0.5 * dense[1:-1] + 0.25 * dense[2:]
    return dense


def _tube(points3: list, X, Y, Z) -> np.ndarray:
    """Distance to a 3D polyline, measured only near it (elsewhere: far)."""
    dense = _dense3(points3)
    shape = np.broadcast_shapes(X.shape, Y.shape, Z.shape)
    pts = np.stack([np.broadcast_to(a, shape).ravel() for a in (X, Y, Z)], axis=1)
    dist = cKDTree(dense).query(pts, distance_upper_bound=40.0)[0]
    return np.minimum(dist, 40.0).reshape(shape).astype(np.float32)


def build_volumes(world: World) -> list[dict]:
    doc, g, diag = world.doc, world.grid, world.diag
    specs = {n: s for n, s in doc.get("field", {}).items() if s.get("type") == "volume"}
    out: list[dict] = []
    if not specs:
        return out
    height = world.field("height")
    world.fields["bed"] = height.copy()
    world.fields["hole"] = np.zeros_like(height)
    for fname, spec in specs.items():
        cell = float(spec.get("cell", 0.75))
        for host in world.features:
            kind = world.kinds.get(host.get("kind", ""), {})
            shape = world.shapes.get(host.get("id"))
            if kind.get("hosts") != fname or shape is None or shape.type != "region":
                continue
            hid = host["id"]
            x0, y0, x1, y1 = shape.bbox
            z0, z1 = [float(v) for v in host["z"]]
            xs = np.arange(x0, x1 + cell * 0.5, cell, dtype=np.float32)
            ys = np.arange(y0, y1 + cell * 0.5, cell, dtype=np.float32)
            zs = np.arange(z0, z1 + cell * 0.5, cell, dtype=np.float32)
            X = xs[None, None, :]
            Y = ys[None, :, None]
            Z = zs[:, None, None]
            X2, Y2 = np.meshgrid(xs, ys)
            ground = (g.sample(height, X2, Y2).astype(np.float32))[None] - Z            # > 0 is solid
            host_sd = g.sample(shape.frame(1e9)["sd"], X2, Y2).astype(np.float32)

            def env_for(extra: dict) -> dict:
                env = {
                    "__builtins__": {}, "x": X, "y": Y, "z": Z, "ground": ground,
                    "min": np.minimum, "max": np.maximum, "abs": np.abs, "clamp": np.clip, "where": np.where,
                    "smoothstep": smoothstep, "mix": lambda a, b, t: a + (b - a) * t, "sin": np.sin, "cos": np.cos,
                    "sqrt": np.sqrt,
                    "fbm3": lambda wavelength, octaves=3, seed=0: _fbm3(X, Y, Z, world.seed * 7 + seed, wavelength, octaves),
                    "fbm2": lambda wavelength, octaves=2, seed=0: sum(
                        0.5 ** o * _perlin(X2 * (2 ** o) / wavelength, Y2 * (2 ** o) / wavelength, world.seed * 13 + seed * 7 + o)
                        for o in range(int(octaves)))[None].astype(np.float32),
                    "f": lambda name: g.sample(world.field(name), X2, Y2).astype(np.float32)[None],
                    "steep": np.clip(np.degrees(np.arctan(np.hypot(*np.gradient(g.sample(height, X2, Y2), cell)))), 0, 90)[None].astype(np.float32),
                }
                # the highest ground within a few metres: nothing an effect adds may stand above it
                env["crest"] = maximum_filter(g.sample(height, X2, Y2).astype(np.float32), size=int(10.0 / cell) | 1)[None] - Z
                env.update(world.params)
                env.update(extra)
                return env

            def ev(expr, env, what):
                if isinstance(expr, (int, float)):
                    return np.float32(expr)
                try:
                    return eval(" ".join(str(expr).split()), env)
                except Unresolved as exc:       # a sound expression that names something undeclared
                    world.report_unresolved(exc, what, expr)
                    return ground
                except Exception as exc:
                    diag.error("expr.invalid", what, f"cannot evaluate `{' '.join(str(expr).split())[:80]}`: {exc}")
                    return ground

            v = np.broadcast_to(np.asarray(ev(spec.get("base", "ground"), env_for({}), f"field.{fname}"), dtype=np.float32),
                                ground.shape).astype(np.float32, copy=True)
            for kname, k in world.kinds.items():
                for eff in k.get("effect", []):
                    if eff.get("field") != fname:
                        continue
                    for feat in world.features:
                        fshape = world.shapes.get(feat.get("id"))
                        planless = k.get("plan") is False
                        if feat.get("kind") != kname or (fshape is None and not planless):
                            continue
                        if k.get("hosts") == fname and feat is not host:
                            continue      # a host's own effect shapes its own box, not its neighbours'
                        if "path3" in feat:
                            q = np.asarray(feat["path3"], dtype=np.float64)
                            if not (g.sample(shape.frame(1e9)["sd"], q[:, 0], q[:, 1]) < 0).any():
                                continue  # this path never enters this volume
                        local = dict(world.attrs(feat))
                        local["v"] = v
                        if fshape is not None:
                            fr = fshape.frame(1e9)
                            local.update({"d": g.sample(fr["d"], X2, Y2).astype(np.float32)[None],
                                          "sd": g.sample(fr["sd"], X2, Y2).astype(np.float32)[None]})
                        if "path3" in feat:
                            local["tube"] = _tube(feat["path3"], X, Y, Z)
                            dense = _dense3(feat["path3"])
                            pd, idx = cKDTree(dense[:, :2]).query(np.stack([X2.ravel(), Y2.ravel()], axis=1))
                            local["pd"] = pd.reshape(X2.shape).astype(np.float32)[None]          # distance in plan
                            local["pz"] = dense[idx, 2].reshape(X2.shape).astype(np.float32)[None]  # the path's height there
                            local["t"] = (idx / max(len(dense) - 1, 1)).reshape(X2.shape).astype(np.float32)[None]
                            p3 = np.asarray(feat["path3"], dtype=np.float64)
                            sd_at = g.sample(shape.frame(1e9)["sd"], p3[:, 0], p3[:, 1])
                            inside = (sd_at < 0) & (p3[:, 2] > z0) & (p3[:, 2] < z1)
                            if not inside.all():
                                k_out = int(np.argmin(inside))
                                diag.warn("volume.outside", feat["id"],
                                          f"it leaves the volume '{hid}' at ({p3[k_out, 0]:g}, {p3[k_out, 1]:g}, {p3[k_out, 2]:g}); "
                                          "nothing is carved outside a volume", where=p3[k_out, :2],
                                          hint="grow the volume's region or z range, or shorten the path")
                        env = env_for(local)
                        for key, expr in k.get("let", {}).items():
                            env[key] = ev(expr, env, f"kind.{kname}.let.{key}")
                        val = np.asarray(ev(eff["value"], env, f"kind.{kname}"), dtype=np.float32)
                        mask = np.clip(np.asarray(ev(eff.get("mask", 1.0), env, f"kind.{kname}"), dtype=np.float32), 0, 1)
                        v = v + (val - v) * mask
            # the apron: at the sides of the box the volume is the ground again
            free = bool(host.get("free"))      # an object, not land: no apron, no floor, and the ground is left alone
            if not free:
                apron = float(host.get("apron", 6.0))
                w = smoothstep(0.0, apron, -host_sd)[None]
                v = (ground + (v - ground) * w).astype(np.float32)

            solid = v > 0
            if not free and (~solid[0]).any():
                j, i = np.argwhere(~solid[0])[0]
                diag.error("volume.floor", hid, f"air reaches the bottom of the volume at z = {z0:g} m",
                           where=(xs[i], ys[j]), hint="lower the volume's z range, or carve less deeply")
            if solid[-1].any():
                j, i = np.argwhere(solid[-1])[0]
                diag.error("volume.ceiling", hid, f"solid reaches the top of the volume at z = {z1:g} m",
                           where=(xs[i], ys[j]), hint="raise the volume's z range")

            verts, faces, _, _ = marching_cubes(v, level=0.0, spacing=(cell, cell, cell))
            grad = np.gradient(v, cell)
            at = (verts / cell).T
            n = np.stack([map_coordinates(grad[k], at, order=1, mode="nearest") for k in range(3)], axis=1)
            n = -n / np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-6)       # toward the air
            air = ~solid
            open_sky = air & (np.cumsum(solid[::-1], axis=0)[::-1] == 0)
            near = gaussian_filter(open_sky.astype(np.float32), (2.5, 5.0, 5.0)) / (gaussian_filter(air.astype(np.float32), (2.5, 5.0, 5.0)) + 1e-3)
            wide = gaussian_filter(open_sky.astype(np.float32), (4.0, 16.0, 16.0)) / (gaussian_filter(air.astype(np.float32), (4.0, 16.0, 16.0)) + 1e-3)
            sky = np.clip(np.maximum(near, 0.7 * wide), 0.0, 1.0)
            light = map_coordinates(sky, (verts / cell).T + (n.T * 1.2), order=1, mode="nearest").astype("<f4")
            world_v = np.stack([verts[:, 2] + x0, verts[:, 1] + y0, verts[:, 0] + z0], axis=1).astype("<f4")
            world_n = np.stack([n[:, 2], n[:, 1], n[:, 0]], axis=1).astype("<f4")

            # top of the solid, and the top of its lowest run, per column
            nz = len(zs)
            k_top = nz - 1 - np.argmax(solid[::-1], axis=0)
            jj, ii = np.indices(k_top.shape)
            va = v[k_top, jj, ii]
            vb = v[np.minimum(k_top + 1, nz - 1), jj, ii]
            top = zs[k_top] + cell * np.where(va - vb > 1e-6, va / np.maximum(va - vb, 1e-6), 0.0)
            first_air = np.argmax(air, axis=0)                 # first air cell from the bottom
            k_bed = np.maximum(first_air - 1, 0)
            va = v[k_bed, jj, ii]
            vb = v[np.minimum(k_bed + 1, nz - 1), jj, ii]
            bed = zs[k_bed] + cell * np.where(va - vb > 1e-6, va / np.maximum(va - vb, 1e-6), 0.0)

            win = g.window(shape.bbox, 0.0)
            WX, WY = g.X[win], g.Y[win]
            coords = np.stack([(WY - y0) / cell, (WX - x0) / cell])
            inside = shape.frame(1e9)["sd"][win] < 0
            new_top = map_coordinates(top, coords, order=1, mode="nearest")
            new_bed = map_coordinates(bed, coords, order=1, mode="nearest")
            lift, under = 0.0, 0.0
            if not free:
                lift = float(np.abs(np.where(inside, new_top - height[win], 0.0)).max())
                height[win] = np.where(inside, new_top, height[win])
                world.fields["bed"][win] = np.where(inside, np.minimum(new_bed, new_top), world.fields["bed"][win])
                world.fields["hole"][win] = np.where(shape.frame(1e9)["sd"][win] < -cell, 1.0, world.fields["hole"][win])
                under = float(((new_top - new_bed) > 1.5)[inside].mean()) if inside.any() else 0.0
            out.append({"id": hid, "field": fname, "verts": world_v, "normals": world_n, "faces": faces.astype("<u4"), "light": light,
                        "cells": [len(xs), len(ys), len(zs)], "cell_m": cell, "tris": int(len(faces)),
                        "overhang_share": round(under, 3), "top_moved_m": round(lift, 2)})
            print(f"  volume {hid}: {len(xs)}x{len(ys)}x{len(zs)} cells of {cell:g} m, {len(faces):,} triangles, "
                  f"{under * 100:.0f}% of its footprint has open space under solid ground")
    return out
