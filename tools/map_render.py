"""Draw a player map's layers (lib/map's export) beside the build it came from, and measure their
registration. A review tool: no reader of the map needs it.

    python3 tools/map_render.py BUILD MAPDIR --out PREFIX [--ppm N] [--expect EXPECT [--section T]]
                                [--against OLDBUILD OLDMAPDIR]

Writes PREFIX.ground.png (left: the map's ground classes with its ways, footprints, regions and
labels, each label in the box the map placed it in;
right: the build's own plan view, the height field shaded, with features.json's ways and floors
drawn over the map's classes in black) and, when the map has floors, PREFIX.floors.png (one plan
per floor, then a section: each floor's surfaces at their levels and each link from its foot to its
head). Prints the registration measures as JSON (protocol in the README of lib/map).

`--ppm` defaults to the map's own label zoom (labels.json `placement`), the scale its labels are
kept apart at; the floors are drawn at the floors' label zoom. Measures:

- `way_on_cells` (ways) and `water_on_cells` (water courses), apart: every 0.25 m along the line
  in features.json, the share whose map cell is its own class, another way or water course, or a
  footprint. A way marked `under` (a cave) is not in the raster; it is listed in `skipped` with
  that reason, never left out silently.
- `label_overlaps`: pairs of placed labels whose text, as this tool draws it (PIL's default font
  at the plan's zoom), meets; `labels_unplaced`: labels with no box.
- With `--against`: `tiles_changed`, and each changed tile explained by what changed in it (the
  raster's cells, or the names of the items whose part in it differs); `tiles_unexplained` counts
  any left, which is a fault of the fingerprint; `tiles_cells_only`, those whose only reason is
  changed cells (a field moved under no changed item).
"""

import argparse
import json
import math
import os
import runpy

import numpy as np
from PIL import Image, ImageDraw

PALETTE = {"none": (236, 232, 222), "water": (120, 170, 220), "footprint": (150, 120, 100)}
GROUND = [(186, 214, 150), (226, 210, 160), (170, 160, 150), (200, 200, 180), (160, 190, 130)]
NAMED = {"grass": (186, 214, 150), "sand": (232, 214, 160), "rock": (165, 160, 155)}
WAYS = [(250, 250, 250), (240, 200, 120), (230, 150, 90), (90, 130, 200)]
# Ways by class (M7): darker than every ground colour above, so an earth road on grass reads (it
# was white on pale green, faint at the map's zoom); a class not named here takes WAYS in turn.
WAY_NAMED = {"road": (176, 98, 44), "trail": (150, 96, 60), "path": (128, 84, 56), "lane": (196, 128, 64),
             "street": (120, 116, 112), "freeway": (214, 120, 40), "bridge": (90, 90, 90)}


def colours(classes):
    out, g, w = {}, 0, 0
    for c in classes:
        n = c["name"]
        if n in PALETTE:
            out[c["id"]] = PALETTE[n]
        elif n.startswith("ground:"):
            out[c["id"]] = NAMED.get(n[7:], GROUND[g % len(GROUND)])
            g += 1
        elif n.startswith("water:"):
            out[c["id"]] = (70, 120, 200)
        elif n.startswith("way:") and n[4:] in WAY_NAMED:
            out[c["id"]] = WAY_NAMED[n[4:]]
        else:
            out[c["id"]] = WAYS[w % len(WAYS)]
            w += 1
    return out


HERE = os.path.dirname(os.path.abspath(__file__))
PLACE = runpy.run_path(os.path.join(HERE, "..", "lib", "map", "export.py"))["place_labels"]
LABEL_INK = {"region": (110, 0, 150), "poi": (170, 0, 0), "label": (0, 0, 0), "room": (0, 0, 120)}


def draw_labels(d, labels, px, ppm):
    """Each label's anchor, and its text in its placed box (top-left corner, plus 1 px of the
    box's padding); returns the drawn text boxes in pixels, as the font measures them."""
    out = []
    for lb in labels:
        x, y = px(lb["at"])
        ink = LABEL_INK.get(lb["class"], (0, 0, 0))
        d.ellipse((x - 1.5, y - 1.5, x + 1.5, y + 1.5), fill=ink)
        if not lb.get("box"):
            continue
        b = lb["box"]
        tx, ty = px((b[0], b[3]))
        tx, ty = tx + 1, ty + 1
        d.text((tx, ty), lb["text"], fill=ink)
        out.append((lb["key"], d.textbbox((tx, ty), lb["text"])))
    return out


def overlaps(boxes):
    n = 0
    for i, (_, a) in enumerate(boxes):
        for _, b in boxes[i + 1:]:
            n += a[0] < b[2] and b[0] < a[2] and a[1] < b[3] and b[1] < a[3]
    return n


def explain_tiles(old_dir, new_dir):
    """Every tile whose fingerprint differs between two maps, with what differs in it."""
    def load(d):
        idx = json.load(open(os.path.join(d, "map.json")))
        vec = json.load(open(os.path.join(d, "vectors.json")))
        lab = json.load(open(os.path.join(d, "labels.json")))["labels"]
        items = {}
        for k in ("ways", "water", "footprints", "regions"):
            for e in vec[k]:
                items[("ground", k, e["name"])] = e
        for fl in idx["floors"]:
            fd = json.load(open(os.path.join(d, fl["file"])))
            for k in ("surfaces", "rooms", "walls", "doors", "links"):
                for e in fd[k]:
                    items[(f"floor:{fl['index']}", k, e.get("name") or e.get("instance"))] = e
        for e in lab:
            plan = "ground" if e["floor"] is None else f"floor:{e['floor']}"
            items[(plan, "labels", e["key"])] = e
        return idx, np.load(os.path.join(d, "ground.npy")), items

    oi, og, oit = load(old_dir)
    ni, ng, nit = load(new_dir)
    T = ni["tiles"]["side"]
    cell = ni["transform"]["cell"]
    x0, y0 = ni["transform"]["extent"][:2]
    of = {(t["plan"], tuple(t["at"])): t["fingerprint"] for t in oi["tiles"]["list"]}
    changed_items = [k for k in set(oit) | set(nit) if oit.get(k) != nit.get(k)]

    def reaches(e, tb):
        if e is None:
            return False
        pts = list(e.get("points", [])) + list(e.get("outline", [])) + [p for pc in e.get("pieces", []) for p in pc[0]]
        if "at" in e:
            pts.append(e["at"])
        if e.get("box"):
            pts += [e["box"][:2], e["box"][2:]]
        if not pts:
            return False
        g = e.get("width", 0) / 2
        bx = (min(p[0] for p in pts) - g, min(p[1] for p in pts) - g,
              max(p[0] for p in pts) + g, max(p[1] for p in pts) + g)
        return bx[0] < tb[2] and tb[0] <= bx[2] and bx[1] < tb[3] and tb[1] <= bx[3]

    out = []
    for t in ni["tiles"]["list"]:
        key = (t["plan"], tuple(t["at"]))
        if of.get(key) == t["fingerprint"]:
            continue
        tx, ty = t["at"]
        why = []
        if t["plan"] == "ground":
            sl = (slice(ty * T, (ty + 1) * T), slice(tx * T, (tx + 1) * T))
            nd = int((og[sl] != ng[sl]).sum())
            if nd:
                why.append(f"cells {nd}")
        tb = (x0 + tx * T * cell, y0 + ty * T * cell, x0 + (tx + 1) * T * cell, y0 + (ty + 1) * T * cell)
        why += sorted(f"{k}:{n}" for pl, k, n in changed_items if pl == t["plan"] and
                      (reaches(oit.get((pl, k, n)), tb) or reaches(nit.get((pl, k, n)), tb)))
        out.append({"plan": t["plan"], "at": [tx, ty], "why": why})
    return out


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("build")
    ap.add_argument("mapdir")
    ap.add_argument("--out", required=True)
    ap.add_argument("--ppm", type=float, help="pixels per metre; default the map's label zoom")
    ap.add_argument("--expect", help="an expect.toml whose [map] rows the measures are checked against")
    ap.add_argument("--section", default="map", help="the expect.toml table of rows (default `map`)")
    ap.add_argument("--against", nargs=2, metavar=("OLDBUILD", "OLDMAPDIR"),
                    help="an earlier build's map: explain every tile whose fingerprint changed")
    a = ap.parse_args(argv)
    idx = json.load(open(os.path.join(a.mapdir, "map.json")))
    vec = json.load(open(os.path.join(a.mapdir, "vectors.json")))
    ldoc = json.load(open(os.path.join(a.mapdir, "labels.json")))
    labels = ldoc["labels"]
    zoom = ldoc.get("placement", {}).get("zoom", {"ground": 4.0, "floors": 14.0})
    glyph = ldoc.get("placement", {}).get("glyph", [7, 13])
    ground = np.load(os.path.join(a.mapdir, "ground.npy"))
    man = json.load(open(os.path.join(a.build, "manifest.json")))
    fj = json.load(open(os.path.join(a.build, "features.json")))
    tr = idx["transform"]
    x0, y0, x1, y1 = tr["extent"]
    cell = tr["cell"]
    ppm = a.ppm or zoom["ground"]
    W, H = int(round((x1 - x0) * ppm)), int(round((y1 - y0) * ppm))

    def px(p):
        return ((p[0] - x0) * ppm, (y1 - p[1]) * ppm)

    col = colours(idx["classes"])
    rgb = np.zeros(ground.shape + (3,), dtype=np.uint8)
    for k, c in col.items():
        rgb[ground == k] = c
    base = Image.fromarray(rgb[::-1]).resize((W, H), Image.NEAREST)

    # Left: the map as data, drawn.
    left = base.copy()
    d = ImageDraw.Draw(left)
    for w in vec["ways"]:
        pts = [px(p) for p in w["points"]]
        d.line(pts, fill=(200, 60, 160) if w["under"] else (60, 60, 60), width=1)
    for w in vec["water"]:
        d.line([px(p) for p in w["points"]], fill=(30, 80, 160), width=1)
    for f in vec["footprints"]:
        for pc in f["pieces"]:
            d.polygon([px(p) for p in pc[0]], outline=(60, 30, 20))
    for r in vec["regions"]:
        for pc in r["pieces"]:
            d.polygon([px(p) for p in pc[0]], outline=(120, 0, 160))
    drawn = draw_labels(d, [lb for lb in labels if lb["floor"] is None], px, ppm)
    d.text((4, 4), f"map: {len(idx['classes'])} classes, cell {cell} m, {ppm} px/m", fill=(0, 0, 0))

    # Right: the build's plan view (height shaded) with features.json drawn over the map classes.
    h = np.load(os.path.join(a.build, man["fields"]["height"]["file"]))
    gy, gx = np.gradient(h.astype(np.float64), man["frame"]["resolution"])
    shade = np.clip(0.75 + 0.35 * (-gx - gy) / np.sqrt(1 + gx * gx + gy * gy), 0, 1)
    sh = Image.fromarray((shade[::-1] * 255).astype(np.uint8)).resize((W, H), Image.BILINEAR).convert("RGB")
    right = Image.blend(sh, base, 0.45)
    d = ImageDraw.Draw(right)
    for f in fj["features"]:
        if f["shape"] == "path" and f["kind"] in {w["kind"] for w in vec["ways"] + vec["water"]}:
            d.line([px(p) for p in f["points"]], fill=(0, 0, 0), width=1)
            wd = f["attrs"].get("width")
            if isinstance(wd, (int, float)):
                for side in (-1, 1):
                    pts = []
                    for i, p in enumerate(f["points"]):
                        q = f["points"][min(i + 1, len(f["points"]) - 1)]
                        o = f["points"][max(i - 1, 0)]
                        dx, dy = q[0] - o[0], q[1] - o[1]
                        n = math.hypot(dx, dy) or 1
                        pts.append(px((p[0] - side * dy / n * wd / 2, p[1] + side * dx / n * wd / 2)))
                    d.line(pts, fill=(0, 0, 0), width=1)
        if f["shape"] == "region" and f["kind"] == "storey":
            for pc in f["pieces"]:
                d.polygon([px(p) for p in pc["outer"]], outline=(0, 0, 0))
    d.text((4, 4), "build: height shaded, features.json in black", fill=(0, 0, 0))
    sheet = Image.new("RGB", (2 * W + 8, H), (255, 255, 255))
    sheet.paste(left, (0, 0))
    sheet.paste(right, (W + 8, 0))
    sheet.save(a.out + ".ground.png")

    # Registration: features.json's ways and floors sampled against the map's cells.
    by_name = {c["name"]: c["id"] for c in idx["classes"]}
    nx, ny = tr["size"]

    def cls_at(x, y):
        c = int(math.floor((x - x0) / cell))
        r = int(math.floor((y - y0) / cell))
        return int(ground[min(max(r, 0), ny - 1), min(max(c, 0), nx - 1)])

    fby = {f["name"]: f for f in fj["features"]}
    meas = {"ways": {}, "water": {}}
    skipped = {}
    for group, items in (("ways", vec["ways"]), ("water", vec["water"])):
        for w in items:
            if w.get("under"):
                skipped[w["name"]] = "under: in vectors.json only, not drawn on the ground raster"
                continue
            f = fby[w["name"]]
            want = by_name.get("way:" + w["class"], by_name.get("water:" + w["class"]))
            hit = tot = 0
            pts = f["points"]
            for i in range(len(pts) - 1):
                L = math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1])
                n = max(1, int(L / 0.25))
                for k in range(n):
                    t = k / n
                    x = pts[i][0] + t * (pts[i + 1][0] - pts[i][0])
                    y = pts[i][1] + t * (pts[i + 1][1] - pts[i][1])
                    if not (x0 <= x < x1 and y0 <= y < y1):
                        continue
                    tot += 1
                    c = cls_at(x, y)
                    hit += c == want or idx["classes"][c]["name"] in ("footprint",) or idx["classes"][c]["from"] in ("way", "water course")
            meas[group][w["name"]] = round(hit / tot, 4) if tot else None
    counts = {"classes": len(idx["classes"]), "floors": len(idx["floors"]),
              "tiles": len(idx["tiles"]["list"]), "ways": len(vec["ways"]), "water": len(vec["water"]),
              "footprints": len(vec["footprints"]), "regions": len(vec["regions"]), "labels": len(labels),
              "under": len(skipped),
              "way_on_cells_min": min((v for v in meas["ways"].values() if v is not None), default=None),
              "water_on_cells_min": min((v for v in meas["water"].values() if v is not None), default=None),
              "labels_unplaced": sum(1 for lb in labels if not lb.get("box")),
              "label_overlaps": overlaps(drawn)}
    links = set()
    for f in idx["floors"]:
        fd = json.load(open(os.path.join(a.mapdir, f["file"])))
        counts["walls"] = counts.get("walls", 0) + len(fd["walls"])
        links |= {ln["name"] for ln in fd["links"]}
    counts["links"] = len(links)
    report = {"way_on_cells": meas["ways"], "water_on_cells": meas["water"], "skipped": skipped,
              "counts": counts}
    if a.against:
        tiles = explain_tiles(a.against[1], a.mapdir)
        report["tiles_changed"] = tiles
        counts["tiles_changed"] = len(tiles)
        counts["tiles_unexplained"] = sum(1 for t in tiles if not t["why"])
        counts["tiles_cells_only"] = sum(1 for t in tiles if t["why"] and all(w.startswith("cells") for w in t["why"]))
    print(json.dumps(report, sort_keys=True))
    if a.expect:
        text = open(a.expect).read()
        try:
            import tomllib

            rows = tomllib.loads(text)[a.section]
        except ImportError:  # Python before 3.11: the rows' one form, read by pattern
            import re

            head = re.search(r"^\[" + re.escape(a.section) + r"\][ \t]*(#.*)?$", text, re.M)
            part = text[head.end():].split("\n[", 1)[0]
            rows = {m.group(1): {"band": [float(m.group(2)), float(m.group(3))]} for m in
                    re.finditer(r"^(\w+)\s*=\s*\{\s*band\s*=\s*\[([-\d.]+),\s*([-\d.]+)\]", part, re.M)}
        bad = [f"{k} {counts.get(k)} not in {r['band']}" for k, r in rows.items()
               if counts.get(k) is None or not r["band"][0] <= counts[k] <= r["band"][1]]
        print("map rows: " + ("; ".join(bad) if bad else f"{len(rows)} in band"))
        if bad:
            return 1

    if not idx["floors"]:
        return 0
    floors = [json.load(open(os.path.join(a.mapdir, f["file"]))) for f in idx["floors"]]
    allpts = [p for fl in floors for s in fl["surfaces"] for pc in s["pieces"] for p in pc[0]]
    bx0, by0 = min(p[0] for p in allpts) - 3, min(p[1] for p in allpts) - 3
    bx1, by1 = max(p[0] for p in allpts) + 3, max(p[1] for p in allpts) + 3
    fp = zoom["floors"]
    FW, FH = int((bx1 - bx0) * fp), int((by1 - by0) * fp)

    def fq(p):
        return ((p[0] - bx0) * fp, (by1 - p[1]) * fp)

    panels = []
    for fl in floors:
        im = Image.new("RGB", (FW, FH + 16), (255, 255, 255))
        d = ImageDraw.Draw(im)
        for s in fl["surfaces"]:
            for pc in s["pieces"]:
                d.polygon([fq(p) for p in pc[0]], fill=(235, 228, 205), outline=(160, 150, 120))
        for r in fl["rooms"]:
            for pc in r["pieces"]:
                d.polygon([fq(p) for p in pc[0]], outline=(120, 120, 200))
        for w in fl["walls"]:
            d.polygon([fq(p) for p in w["outline"]], fill=(50, 50, 50))
        for dr in fl["doors"]:
            d.line([fq(p) for p in dr["points"]], fill=(0, 160, 0), width=3)
        notes = []
        for ln in fl["links"]:
            colr = (230, 120, 0) if ln["to"] != ln["from"] else (160, 160, 0)
            d.line([fq(p) for p in ln["points"]], fill=colr, width=3)
            end = ln["points"][-1] if ln["from"] == fl["index"] else ln["points"][0]
            other = ln["to"] if ln["from"] == fl["index"] else ln["from"]
            word = "up" if other > fl["index"] else "down" if other < fl["index"] else ""
            notes.append({"key": "~" + ln["name"], "class": "link", "text": f"{word} {other}".strip(),
                          "at": end, "floor": fl["index"]})
        # The links' notes are this tool's, not the map's: placed by the map's own rule after its
        # labels (which keep their boxes: they come first in its order), so neither covers the other.
        mine = [dict(lb) for lb in labels if lb["floor"] == fl["index"]]
        PLACE(mine + notes, fp, glyph, [bx0, by0, bx1, by1])
        fdrawn = draw_labels(d, mine, fq, fp)
        moved = sum(1 for lb, m in zip([lb for lb in labels if lb["floor"] == fl["index"]], mine)
                    if lb.get("box") != m.get("box"))
        for nt in notes:
            if nt.get("box"):
                b = nt["box"]
                tx, ty = fq((b[0], b[3]))
                d.text((tx + 1, ty + 1), nt["text"], fill=(230, 120, 0))
                fdrawn.append((nt["key"], d.textbbox((tx + 1, ty + 1), nt["text"])))
        print(json.dumps({"floor": fl["index"], "label_overlaps": overlaps(fdrawn), "labels_moved": moved,
                          "labels_unplaced": sum(1 for lb in labels if lb["floor"] == fl["index"] and not lb.get("box")),
                          "notes_unplaced": sum(1 for nt in notes if not nt.get("box"))}))
        d.text((4, FH + 2), f"floor {fl['index']}: levels {fl['levels']} z {fl['z']}  walls {len(fl['walls'])} "
               f"doors {len(fl['doors'])} links {len(fl['links'])}", fill=(0, 0, 0))
        panels.append(im)
    # Section: x against z.
    zs = [lv for fl in floors for lv in fl["levels"]] + [z for fl in floors for ln in fl["links"] for z in ln["z"]]
    zmax = max(zs) + 2
    SH = int(zmax * 20) + 20
    sec = Image.new("RGB", (FW, SH), (255, 255, 255))
    d = ImageDraw.Draw(sec)

    def sq(x, z):
        return ((x - bx0) * fp, SH - 10 - z * 20)

    for fl in floors:
        lo = fl["z"][0]
        d.line([sq(bx0, lo), sq(bx1, lo)], fill=(200, 200, 255))
        d.text(sq(bx0, lo + 0.1), f"floor {fl['index']} from z {lo}", fill=(100, 100, 200))
        for s in fl["surfaces"]:
            xs = [p[0] for pc in s["pieces"] for p in pc[0]]
            d.line([sq(min(xs), s["level"]), sq(max(xs), s["level"])], fill=(120, 90, 40), width=3)
    seen = set()
    for fl in floors:
        for ln in fl["links"]:
            if ln["name"] in seen:
                continue
            seen.add(ln["name"])
            d.line([sq(ln["points"][0][0], ln["z"][0]), sq(ln["points"][-1][0], ln["z"][1])], fill=(230, 120, 0), width=2)
    d.text((4, 4), "section: floor surfaces at their levels (brown), links foot to head (orange), x east", fill=(0, 0, 0))
    cols = 2
    rows = (len(panels) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * (FW + 8), rows * (FH + 24) + SH), (255, 255, 255))
    for i, im in enumerate(panels):
        sheet.paste(im, ((i % cols) * (FW + 8), (i // cols) * (FH + 24)))
    sheet.paste(sec, (0, rows * (FH + 24)))
    sheet.save(a.out + ".floors.png")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
