#!/usr/bin/env python3
"""Composite legend panels next to a suitability run's rendered maps.

Presentation-only: the via-viz renderers are deliberately font-free
(captions live in reports), so this sidecar script reads the PNGs the
CLI rendered plus the stage summary (`suitability.<label>.json`) and
writes `*_labeled.png` beside them, with site counts and the declared
config values pulled from the summary — nothing is typed twice.

Usage (after `via suitability <run_dir>`):

    uv run --project analysis python analysis/label_suitability_panels.py \
        <run_dir> [--label site] [--out <dir>]
"""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

PANEL_W = 460
BG = (24, 26, 32)
FG = (235, 235, 235)
DIM = (165, 168, 178)

FONT_CANDIDATES = [
    "/System/Library/Fonts/Helvetica.ttc",  # macOS
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",  # Linux
]


def load_fonts():
    for path in FONT_CANDIDATES:
        try:
            return {
                "title": ImageFont.truetype(path, 26),
                "head": ImageFont.truetype(path, 19),
                "body": ImageFont.truetype(path, 16),
                "small": ImageFont.truetype(path, 13),
            }
        except OSError:
            continue
    f = ImageFont.load_default()
    return {"title": f, "head": f, "body": f, "small": f}


FONTS = load_fonts()


def wrap(draw, text, font, max_w):
    words, lines, cur = text.split(), [], ""
    for wd in words:
        trial = (cur + " " + wd).strip()
        if draw.textlength(trial, font=font) <= max_w:
            cur = trial
        else:
            lines.append(cur)
            cur = wd
    if cur:
        lines.append(cur)
    return lines


def swatch(draw, x, y, colors):
    wchip = 26 if len(colors) == 1 else 18
    for k, c in enumerate(colors):
        draw.rectangle([x + k * wchip, y, x + (k + 1) * wchip - 2, y + 18], fill=c)
    return x + len(colors) * wchip + 10


def build(src: Path, out: Path, title, subtitle, entries, footnotes):
    img = Image.open(src).convert("RGB")
    # Small maps are upscaled so the 1-cell markers read; large ones are
    # already legible and upscaling would dwarf the legend panel.
    if img.width < 800:
        img = img.resize((img.width * 2, img.height * 2), Image.NEAREST)
    canvas = Image.new("RGB", (img.width + PANEL_W, img.height), BG)
    canvas.paste(img, (0, 0))
    d = ImageDraw.Draw(canvas)
    x0, y = img.width + 24, 26
    maxw = PANEL_W - 48

    d.text((x0, y), title, font=FONTS["title"], fill=FG)
    y += 38
    for ln in wrap(d, subtitle, FONTS["small"], maxw):
        d.text((x0, y), ln, font=FONTS["small"], fill=DIM)
        y += 18
    y += 14

    for heading, items in entries:
        d.text((x0, y), heading, font=FONTS["head"], fill=FG)
        y += 28
        for colors, label in items:
            tx = swatch(d, x0, y, colors)
            lines = wrap(d, label, FONTS["body"], maxw - (tx - x0))
            for k, ln in enumerate(lines):
                d.text((tx, y - 1 + k * 19), ln, font=FONTS["body"], fill=FG)
            y += max(24, 19 * len(lines) + 6)
        y += 12

    y += 6
    for note in footnotes:
        for ln in wrap(d, "• " + note, FONTS["small"], maxw):
            d.text((x0, y), ln, font=FONTS["small"], fill=DIM)
            y += 17
        y += 4

    canvas.save(out)
    print(f"wrote {out}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dir", type=Path)
    ap.add_argument("--label", default="site", help="suitability config label")
    ap.add_argument("--out", type=Path, help="output dir (default: <run_dir>/render)")
    args = ap.parse_args()

    summary = json.loads((args.run_dir / f"suitability.{args.label}.json").read_text())
    aff = summary["affordances"]
    n_conf = len(aff["confluences"]["sites"])
    n_pass = len(aff["passes"]["sites"])
    n_head = len(aff["navigability"]["head_of_navigation_sites"])
    min_pers = aff["passes"]["min_persistence_m"]
    k_q = aff["fords"]["k_q_m3s_per_unit"]

    render = args.run_dir / "render"
    out_dir = args.out or render
    out_dir.mkdir(parents=True, exist_ok=True)

    build(
        render / f"suitability.{args.label}.affordances.png",
        out_dir / f"suitability.{args.label}.affordances_labeled.png",
        "Affordance map",
        "Hillshaded relief: green lowland, brown uplands, white peaks; "
        "ocean blue. Sites and spectra from the via-suitability stage "
        f"(config label '{args.label}').",
        [
            (
                "River tint — crossability D·V (log)",
                [
                    ([(96, 200, 96)], "low hazard — fordable on foot"),
                    ([(230, 190, 60)], "moderate"),
                    ([(150, 30, 30)], "extreme — no ford"),
                ],
            ),
            (
                "Affordance sites",
                [
                    ([(255, 255, 255)], f"confluence ({n_conf}) — river junction"),
                    (
                        [(255, 150, 30)],
                        f"mountain pass ({n_pass}) — saddle ≥ {min_pers:g} m persistence",
                    ),
                    (
                        [(230, 40, 200)],
                        f"head of navigation ({n_head}) — upstream end of the boat-navigable river",
                    ),
                ],
            ),
            (
                "QA overlays (diagnostics, not sites)",
                [
                    (
                        [(120, 20, 20)],
                        "basin-boundary col — where drainage divides dip; the coastal ring is a known artifact",
                    ),
                    (
                        [(40, 220, 220)],
                        "Filet change-point — slope break in a river's long profile",
                    ),
                ],
            ),
        ],
        [
            f"Crossability values are conditional on the declared k_Q scale (here {k_q:g}).",
            "Passes sit on ridgelines; interior dark-red cols near orange passes are the expected correspondence.",
        ],
    )

    build(
        render / f"suitability.{args.label}.harbour.png",
        out_dir / f"suitability.{args.label}.harbour_labeled.png",
        "Harbour components",
        "Land is dimmed relief; the colored ring is coastal water (ocean "
        "cells touching land). Open ocean is left dark — fetch is only "
        "computed on the coast.",
        [
            (
                "Coastal ring — wave fetch",
                [
                    ([(120, 240, 160)], "~1–3 km: sheltered — natural-harbour water"),
                    ([(235, 205, 70)], "~10 km: partly sheltered"),
                    ([(70, 140, 200)], "~40 km: exposed"),
                    ([(15, 35, 105)], "100+ km: fully exposed coast"),
                ],
            ),
            (
                "Overlay",
                [
                    (
                        [(220, 60, 30)],
                        "sediment-supply penalty — river-mouth plumes; the infill risk that killed ancient harbours",
                    ),
                ],
            ),
        ],
        [
            "Good anchorages read as yellow-green cells away from red plumes.",
            "The depth-window component ships as its own raster and is not drawn here.",
            "No composite score exists by design: consumers combine the components under declared config.",
        ],
    )


if __name__ == "__main__":
    main()
