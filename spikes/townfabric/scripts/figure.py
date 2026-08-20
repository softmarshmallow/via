#!/usr/bin/env python3
"""Compose the real-vs-spike comparison figure with legible typography.

The panels are rendered by the Rust binary at metric scale; this only
lays them out, labels them, and draws the legend. Throwaway tooling.
"""
import json
from PIL import Image, ImageDraw, ImageFont

FONT = "/System/Library/Fonts/Supplemental/Arial.ttf"
FONT_B = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"
def f(sz, bold=False):
    return ImageFont.truetype(FONT_B if bold else FONT, sz)

PANEL = 560          # displayed size of each map panel
CROP  = 1200         # pixels taken from the source panel (= 600 m at 0.5 m/px)
GUT   = 18
LEFT  = 132          # room for row labels
TOP   = 132
CAPH  = 74           # caption strip under each panel row
LEGH  = 132

INK   = (28, 28, 30)
GREY  = (110, 110, 115)
RED   = (176, 54, 40)

# Colours copied from render.rs so the legend cannot drift from the maps.
SWATCH = [
    ((246, 242, 234), "Street surface", "width follows the street's class"),
    ((206, 200, 182), "Block interior / back-land", "gardens, yards, unbuilt ground"),
    ((228, 222, 204), "Plot (spike only)", "one household holding"),
    ((122, 78, 66),   "Building footprint", "darker = more storeys"),
    ((150, 178, 200), "Water", "river channel or sea"),
    (RED,             "Zoom window", "area shown in the lot-level figure"),
]

def crop(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    l, t = (w - CROP) // 2, (h - CROP) // 2
    return im.crop((l, t, l + CROP, t + CROP)).resize((PANEL, PANEL), Image.LANCZOS)

def stat(fab, k, fmt="%.2f"):
    v = fab.get(k)
    return "-" if v is None else fmt % v

def build(cols, out, title, subtitle):
    W = LEFT + len(cols) * PANEL + (len(cols) + 1) * GUT
    H = TOP + 2 * (PANEL + CAPH) + GUT + LEGH
    img = Image.new("RGB", (W, H), (255, 255, 255))
    d = ImageDraw.Draw(img)

    d.text((GUT, 22), title, font=f(26, True), fill=INK)
    d.text((GUT, 58), subtitle, font=f(15), fill=GREY)
    d.text((GUT, 78), "Every panel is the same 600 m of ground, drawn at the same scale.",
           font=f(15), fill=GREY)

    for row, (label, sub) in enumerate([
            ("REAL TOWNS", "OpenStreetMap"),
            ("THE SPIKE", "generated")]):
        y = TOP + row * (PANEL + CAPH)
        d.text((GUT, y + 8), label, font=f(17, True), fill=INK)
        d.text((GUT, y + 30), sub, font=f(14), fill=GREY)

    for i, c in enumerate(cols):
        x = LEFT + GUT + i * (PANEL + GUT)
        for row, key in enumerate(["real", "spike"]):
            y = TOP + row * (PANEL + CAPH)
            panel, name, fab = c[key]
            img.paste(crop(panel), (x, y))
            d.rectangle([x, y, x + PANEL - 1, y + PANEL - 1], outline=(200, 200, 205))
            d.text((x, y + PANEL + 6), name, font=f(15, True), fill=INK)
            line = (f"meshedness {stat(fab,'meshedness','%.3f')}   "
                    f"dead ends {100*fab['dead_end_share']:.0f}%   "
                    f"blocks {fab['blocks']}")
            d.text((x, y + PANEL + 26), line, font=f(13), fill=GREY)
            line2 = (f"buildings {fab['buildings']}   "
                     f"setback {stat(fab,'bldg_street_dist_median_m','%.1f')} m   "
                     f"street wall {100*fab['street_wall_share']:.0f}%")
            d.text((x, y + PANEL + 44), line2, font=f(13), fill=GREY)
        d.text((x, TOP - 22), c["header"], font=f(15, True), fill=c.get("colour", INK))

    # Legend
    ly = TOP + 2 * (PANEL + CAPH) + GUT
    d.line([(GUT, ly - 8), (W - GUT, ly - 8)], fill=(215, 215, 220))
    d.text((GUT, ly), "WHAT THE COLOURS MEAN", font=f(14, True), fill=INK)
    colw = (W - 2 * GUT) // 3
    for i, (col, name, note) in enumerate(SWATCH):
        cx = GUT + (i % 3) * colw
        cy = ly + 26 + (i // 3) * 34
        d.rectangle([cx, cy, cx + 26, cy + 18], fill=col, outline=(150, 150, 155))
        d.text((cx + 34, cy - 1), name, font=f(13, True), fill=INK)
        d.text((cx + 34, cy + 14), note, font=f(12), fill=GREY)
    img.save(out)
    print("wrote", out, img.size)

if __name__ == "__main__":
    ref = {n: json.load(open(f"runs/reference/{n}-fabric.json"))["fabric"]
           for n in ["alnwick", "lavenham", "abilene", "levittown"]}
    syn = {t["name"]: t for t in json.load(open("runs/spike-townfabric-s42/townfabric.json"))["towns"]}
    S = "runs/spike-townfabric-s42/"
    med = (S + "medieval-organic-town.png", "spike: medieval town", syn["medieval organic"]["fabric_streets_only"])
    cols = [
      {"header": "MEDIEVAL - CALIBRATED ON THIS TOWN",
       "real": ("runs/reference/alnwick-town.png", "Alnwick, England (organic medieval core)", ref["alnwick"]),
       "spike": med},
      {"header": "MEDIEVAL - HELD OUT (never fitted)", "colour": (26, 96, 60),
       "real": ("runs/reference/lavenham-town.png", "Lavenham, England (medieval wool town)", ref["lavenham"]),
       "spike": med},
      {"header": "FRONTIER PLAT",
       "real": ("runs/reference/abilene-town.png", "Abilene, Kansas (railroad plat, downtown)", ref["abilene"]),
       "spike": (S + "frontier-plat-town.png", "spike: frontier plat", syn["frontier plat"]["fabric_streets_only"])},
      {"header": "MODERN SUBURB",
       "real": ("runs/reference/levittown-town.png", "Levittown, New York (FHA-era suburb)", ref["levittown"]),
       "spike": (S + "modern-suburb-town.png", "spike: modern suburb", syn["modern suburb"]["fabric_streets_only"])},
    ]
    build(cols, "runs/reference/step3-labelled.png",
          "Town fabric: real towns against what the spike generates",
          "All numbers use one protocol: the central 600 m disc, mapped streets only (footpaths and alleys excluded), degree-2 vertices dissolved.")
