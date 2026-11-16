"""Draw a player map's layers (lib/map's export) beside the build it came from, and measure their
registration. A review tool: no reader of the map needs it.

    python3 tools/map_render.py BUILD MAPDIR --out PREFIX [--ppm N]

Writes PREFIX.ground.png (left: the map's ground classes with its ways, footprints and labels;
right: the build's own plan view, the height field shaded, with features.json's ways and floors
drawn over the map's classes in black) and, when the map has floors, PREFIX.floors.png (one plan
per floor, then a section: each floor's surfaces at their levels and each link from its foot to its
head). Prints the registration measures as JSON (protocol in the README of lib/map).
"""

import argparse
import json
import math
import os

import numpy as np
from PIL import Image, ImageDraw

PALETTE = {"none": (236, 232, 222), "water": (120, 170, 220), "footprint": (150, 120, 100)}
GROUND = [(186, 214, 150), (226, 210, 160), (170, 160, 150), (200, 200, 180), (160, 190, 130)]
NAMED = {"grass": (186, 214, 150), "sand": (232, 214, 160), "rock": (165, 160, 155)}
WAYS = [(250, 250, 250), (240, 200, 120), (230, 150, 90), (90, 130, 200)]


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
        else:
            out[c["id"]] = WAYS[w % len(WAYS)]
            w += 1
    return out


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("build")
    ap.add_argument("mapdir")
    ap.add_argument("--out", required=True)
    ap.add_argument("--ppm", type=float, default=4.0)
    ap.add_argument("--expect", help="an expect.toml whose [map] rows the measures are checked against")
    a = ap.parse_args(argv)
    idx = json.load(open(os.path.join(a.mapdir, "map.json")))
    vec = json.load(open(os.path.join(a.mapdir, "vectors.json")))
    labels = json.load(open(os.path.join(a.mapdir, "labels.json")))["labels"]
    ground = np.load(os.path.join(a.mapdir, "ground.npy"))
    man = json.load(open(os.path.join(a.build, "manifest.json")))
    fj = json.load(open(os.path.join(a.build, "features.json")))
    tr = idx["transform"]
    x0, y0, x1, y1 = tr["extent"]
    cell = tr["cell"]
    ppm = a.ppm
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
    for lb in labels:
        if lb["floor"] is None:
            x, y = px(lb["at"])
            d.ellipse((x - 2, y - 2, x + 2, y + 2), fill=(200, 0, 0))
            d.text((x + 3, y - 6), lb["text"], fill=(0, 0, 0))
    d.text((4, 4), f"map: {len(idx['classes'])} classes, cell {cell} m", fill=(0, 0, 0))

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

    meas = {}
    for w in vec["ways"] + vec["water"]:
        if w.get("under"):
            continue
        f = next(f for f in fj["features"] if f["name"] == w["name"])
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
        meas[w["name"]] = round(hit / tot, 4) if tot else None
    counts = {"classes": len(idx["classes"]), "floors": len(idx["floors"]),
              "tiles": len(idx["tiles"]["list"]), "ways": len(vec["ways"]), "water": len(vec["water"]),
              "footprints": len(vec["footprints"]), "labels": len(labels),
              "way_on_cells_min": min((v for v in meas.values() if v is not None), default=None)}
    links = set()
    for f in idx["floors"]:
        fd = json.load(open(os.path.join(a.mapdir, f["file"])))
        counts["walls"] = counts.get("walls", 0) + len(fd["walls"])
        links |= {ln["name"] for ln in fd["links"]}
    counts["links"] = len(links)
    print(json.dumps({"way_centreline_on_way_cells": meas, "counts": counts}, sort_keys=True))
    if a.expect:
        import re

        text = open(a.expect).read().split("\n[map]\n", 1)[1].split("\n[", 1)[0]
        rows = {m.group(1): {"band": [float(m.group(2)), float(m.group(3))]} for m in
                re.finditer(r"^(\w+)\s*=\s*\{\s*band\s*=\s*\[([-\d.]+),\s*([-\d.]+)\]", text, re.M)}
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
    fp = 14.0
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
        for ln in fl["links"]:
            colr = (230, 120, 0) if ln["to"] != ln["from"] else (160, 160, 0)
            d.line([fq(p) for p in ln["points"]], fill=colr, width=3)
            end = ln["points"][-1] if ln["from"] == fl["index"] else ln["points"][0]
            other = ln["to"] if ln["from"] == fl["index"] else ln["from"]
            x, y = fq(end)
            d.text((x + 3, y), f"{'up' if other > fl['index'] else 'down' if other < fl['index'] else ''} {other}", fill=colr)
        for lb in labels:
            if lb["floor"] == fl["index"]:
                x, y = fq(lb["at"])
                d.text((x - 10, y - 5), lb["text"], fill=(0, 0, 120))
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
