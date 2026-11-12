"""Reachability lint, on demand: can a player on foot get from the start to every landmark?

Walkable ground is ground no steeper than a limit and not under deep water. The lint labels the
connected walkable areas, then for every target says whether it shares the start's area, how long the
walk is against the straight line, and, when it is cut off, where the narrowest gap is and what
closes it. It reads the top surface only: caves and anything under an overhang are not seen.
"""
from __future__ import annotations

import math

import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import distance_transform_edt, label

from ..world import World


def lint_reach(world: World, instances: list[dict], out_png) -> None:
    """Checks only what the world states: `[[expect]]` rows, each under a declared `[profile.<name>]`.
    Nothing is assumed. With no profile or no expectation, the lint says so and checks nothing."""
    g, diag, doc = world.grid, world.diag, world.doc
    profiles, expects = doc.get("profile", {}), doc.get("expect", [])
    if not expects:
        diag.info("reach.nothing_expected", "world", "no [[expect]] rows; the reach lint has nothing to check")
        print("  reach: nothing expected, nothing checked")
        return
    step = 2                                            # 1 m cells
    h = world.field("height")[::step, ::step]
    cell = g.res * step
    gy, gx = np.gradient(h, cell)
    slope = np.degrees(np.arctan(np.hypot(gx, gy)))
    water = world.fields.get("water")
    depth = (water[::step, ::step] - h) if water is not None else np.full_like(h, -1.0)
    mask_img = Image.new("L", (h.shape[1], h.shape[0]), 0)            # solid footprints stand in the way
    for inst in instances:
        if inst.get("explicit") and world.pack["asset"][inst["asset"]]["solid"]:
            ImageDraw.Draw(mask_img).polygon([((x - g.x0) / cell, (y - g.y0) / cell) for x, y in inst["poly"]], fill=1)
    built = np.asarray(mask_img, dtype=bool)

    def place(name):
        feat = next((f for f in world.features if f.get("id") == name and "at" in f), None)
        if feat is not None:
            return float(feat["at"][0]), float(feat["at"][1])
        inst = next((i for i in instances if i.get("id") == name), None)
        if inst is not None:        # stand at its door side, just outside the footprint
            th = math.radians(inst["yaw"])
            off = inst["footprint"][1] * 0.5 + 2.5
            return inst["x"] + math.cos(th) * off, inst["y"] + math.sin(th) * off
        return None

    img = None
    met = 0
    for n, ex in enumerate(expects):
        pname = ex.get("by", "")
        prof = profiles.get(pname)
        if prof is None or "max_slope_deg" not in prof:
            diag.error("reach.no_profile", f"expect {n}", f"it names the profile '{pname}', which does not declare max_slope_deg",
                       hint="declare [profile.<name>] with max_slope_deg and max_wade_m; the lint assumes nothing")
            continue
        max_slope, max_wade = float(prof["max_slope_deg"]), float(prof.get("max_wade_m", 0.0))
        walk = (slope <= max_slope) & (depth <= max_wade) & ~built
        for name in ex.get("closed", []):       # "with these openings shut": proves every route passes one of them
            if name not in world.gaps:
                diag.error("reach.unknown", name, "`closed` names something that is not an opening left by `open_at`")
                continue
            cx, cy, wide = world.gaps[name]
            walk &= np.hypot(g.X[::step, ::step] - cx, g.Y[::step, ::step] - cy) > wide / 2 + 2 * cell
        labels, _ = label(walk, structure=np.ones((3, 3)))

        def snap(x, y, radius=8.0):
            j, i = int(round((y - g.y0) / cell)), int(round((x - g.x0) / cell))
            r = int(radius / cell)
            j0, i0 = max(0, j - r), max(0, i - r)
            win = walk[j0:j + r + 1, i0:i + r + 1]
            if not win.any():
                return None
            jj, ii = np.nonzero(win)
            k = int(np.argmin((jj + j0 - j) ** 2 + (ii + i0 - i) ** 2))
            return int(jj[k] + j0), int(ii[k] + i0)

        origin = place(ex.get("from", ""))
        s_cell = snap(*origin) if origin else None
        if s_cell is None:
            diag.error("reach.no_start", f"expect {n}", f"'{ex.get('from')}' is not a place on ground the profile '{pname}' can stand on")
            continue
        home = labels[s_cell]
        from_home = distance_transform_edt(labels != home) * cell
        want = ex.get("reach", True)
        why_note = f" ({ex['why']})" if ex.get("why") else ""
        if img is None:
            img = Image.fromarray((np.stack([np.where(labels == home, 110, 60), np.where(walk, 150, 70),
                                             np.where(depth > max_wade, 150, 70)], axis=-1)[::-1]).astype(np.uint8))
        draw = ImageDraw.Draw(img)

        def px(x, y):
            return ((x - g.x0) / cell, h.shape[0] - (y - g.y0) / cell)

        for tid in ex.get("to", []):
            spot = place(tid)
            if spot is None:
                diag.error("reach.unknown", tid, "the expectation names something that is neither a point feature nor an instance")
                continue
            t_cell = snap(*spot)
            tx, ty = px(*spot)
            reached = t_cell is not None and labels[t_cell] == home
            if reached == want:
                met += 1
                draw.ellipse([tx - 5, ty - 5, tx + 5, ty + 5], fill=(80, 255, 120))
                continue
            draw.ellipse([tx - 5, ty - 5, tx + 5, ty + 5], fill=(255, 60, 60))
            if reached:      # it was meant to be out of reach
                diag.error("reach.not_sealed", tid,
                           f"expected out of reach of '{pname}' from '{ex['from']}'{why_note}, but a route exists over the ground",
                           where=spot, hint="close the way in, or change the expectation")
                continue
            if t_cell is None:
                diag.error("reach.cut_off", tid,
                           f"expected reachable by '{pname}' from '{ex['from']}'{why_note}; there is no ground it can stand on within 8 m "
                           f"(slope limit {max_slope:g} degrees, wading {max_wade:g} m)", where=spot)
                continue
            island = labels == labels[t_cell]
            gap = np.where(island, from_home, np.inf)
            j, i = np.unravel_index(int(np.argmin(gap)), gap.shape)
            wx, wy = g.x0 + i * cell, g.y0 + j * cell
            jj, ii = np.nonzero(labels[max(0, j - 30):j + 31, max(0, i - 30):i + 31] == home)
            why = "steep ground"
            if jj.size:
                k = int(np.argmin((jj - min(30, j)) ** 2 + (ii - min(30, i)) ** 2))
                mj, mi = (j + max(0, j - 30) + jj[k]) // 2, (i + max(0, i - 30) + ii[k]) // 2
                why = "deep water" if depth[mj, mi] > max_wade else ("a building" if built[mj, mi] else f"ground at {slope[mj, mi]:.0f} degrees")
            diag.error("reach.cut_off", tid,
                       f"expected reachable by '{pname}' from '{ex['from']}'{why_note}; no route over the ground. "
                       f"The nearest approach leaves {gap[j, i]:.0f} m closed by {why}", where=(wx, wy),
                       hint="carry a path across the gap, soften the ground, or state that this is intended: reach = false")
            gx_, gy_ = px(wx, wy)
            draw.line([gx_ - 7, gy_ - 7, gx_ + 7, gy_ + 7], fill=(255, 220, 0), width=3)
            draw.line([gx_ - 7, gy_ + 7, gx_ + 7, gy_ - 7], fill=(255, 220, 0), width=3)
    if img is not None:
        img.save(out_png)
    total = sum(len(e.get("to", [])) for e in expects)
    print(f"  reach: {met} of {total} expectations hold (terrain surface only: caves, interiors and bridges are not seen)")
