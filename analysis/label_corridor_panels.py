#!/usr/bin/env python3
"""Composite legend panels next to a corridors run's rendered maps.

Presentation-only sidecar (same contract as label_suitability_panels):
reads the PNGs the CLI rendered plus the stage summary
(`corridors.<label>.json`) and writes `*_labeled.png` beside them.
Counts and config values come from the summary — nothing typed twice.

Usage (after `via corridors <run_dir>`):

    uv run --project analysis python analysis/label_corridor_panels.py \
        <run_dir> [--label site] [--out <dir>]
"""

import argparse
import json
from pathlib import Path

from label_suitability_panels import build


def water_note(ford_pct: float, nav_pct: float, summary: dict) -> str:
    """Describe the water regime the run actually landed in.

    The two ends read very differently and each is worth naming: an
    uncalibrated scale makes every river an impassable wall, while a
    realistic one on a small steep world makes every river a crossable
    brook. Both are honest outputs; neither is a defect.
    """
    kq = summary.get("config", {}).get("k_q_m3s_per_unit")
    scale = (
        "the derived k_Q (from the terrain's own precipitation and cell area)"
        if kq is None
        else f"an overridden k_Q of {kq:g}"
    )
    if ford_pct >= 95:
        regime = (
            "Rivers here are crossable almost everywhere, so they shape routes "
            "through accumulated wading delay rather than by forcing detours to "
            "fords. Ford-seeking as a corridor mechanism needs rivers large "
            "enough to be impassable in their lower reaches."
        )
    elif ford_pct <= 5:
        regime = (
            "Rivers here are effectively walls, so cross-river movement detours "
            "via headwater ridges or the coast. On an uncalibrated scale that "
            "shape is an artifact of the constant, not terrain truth."
        )
    else:
        regime = (
            "Rivers are crossable in their upper reaches and impassable lower "
            "down, so fords become genuine route-controlling sites."
        )
    nav = (
        f" No reach is navigable ({nav_pct:.1f}%): these channels are too steep, "
        "which is the physically correct answer for mountainous terrain."
        if nav_pct < 0.5
        else f" {nav_pct:.1f}% of channel cells are navigable."
    )
    return (
        f"Water regime under {scale}: {ford_pct:.1f}% of channel cells are "
        f"fordable. {regime}{nav}"
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dir", type=Path)
    ap.add_argument("--label", default="site", help="corridors config label")
    ap.add_argument("--out", type=Path, help="output dir (default: <run_dir>/render)")
    args = ap.parse_args()

    summary = json.loads((args.run_dir / f"corridors.{args.label}.json").read_text())
    cfg = summary["config"]
    deg = summary["degeneracy"]
    lat = summary["lattice"]
    nodes = summary["trunk"]["nodes"]
    n_pass = sum(1 for n in nodes if n["class"] == "pass")
    n_head = sum(1 for n in nodes if n["class"] == "head_of_navigation")
    n_mouth = sum(1 for n in nodes if n["class"] == "river_mouth")
    n_edges = len(summary["trunk"]["edges"])
    n_junctions = len(summary["trunk"]["junctions"])
    ford_pct = 100.0 * deg["ford_passable_channel_fraction"]
    nav_pct = 100.0 * deg["navigable_channel_fraction"]

    render = args.run_dir / "render"
    out_dir = args.out or render
    out_dir.mkdir(parents=True, exist_ok=True)

    build(
        render / f"corridors.{args.label}.png",
        out_dir / f"corridors.{args.label}_labeled.png",
        "Corridor map (era 0)",
        "Hillshaded relief base. Movement model: Tobler walking time "
        "with the switchback envelope, canoe on navigable water, flat "
        "transshipment at mode switches (ADR 0012). "
        f"{lat['sources']} lattice points, {lat['directed_pairs']} "
        "directed least-cost journeys.",
        [
            (
                "Corridor density (top quintile)",
                [
                    ([(255, 230, 90)], "traversed by many journeys"),
                    ([(255, 110, 20)], "heavily traversed"),
                    ([(225, 0, 70)], "the busiest corridors (log scale)"),
                ],
            ),
            (
                "Trunk network",
                [
                    ([(30, 20, 15)], f"trunk route, land leg ({n_edges} edges)"),
                    ([(20, 60, 160)], "trunk route, water leg"),
                    ([(40, 220, 220)], f"junction, degree 3 or more ({n_junctions})"),
                ],
            ),
            (
                "Gateway nodes (terrain-derived)",
                [
                    ([(255, 150, 30)], f"mountain pass ({n_pass})"),
                    ([(230, 40, 200)], f"head of navigation ({n_head})"),
                    ([(255, 255, 255)], f"river mouth ({n_mouth})"),
                ],
            ),
        ],
        [
            water_note(ford_pct, nav_pct, summary),
            "Density shows only the top 20% of traversed cells (White & "
            "Barber's 80/20 rendering default); the raster ships raw.",
            f"reuse discount alpha = {cfg['reuse_alpha']:g}; ford caps "
            f"D*V <= {cfg['ford_max_dv_m2s']:g} m^2/s, depth <= "
            f"{cfg['ford_max_depth_m']:g} m, velocity <= "
            f"{cfg['ford_max_velocity_ms']:g} m/s — the traveller envelope, "
            "judged at the declared crossing flow.",
        ],
    )

    build(
        render / f"corridors.{args.label}.hours_to_sea.png",
        out_dir / f"corridors.{args.label}.hours_to_sea_labeled.png",
        "Hours to tidewater",
        "Accumulated multimodal travel time from the coast (land-node "
        "projection): Tobler walking with the switchback envelope, plus "
        "canoe and coastal legs where admissible. Rivers draw as dark "
        "lines on the relief pass.",
        [
            (
                "Travel time bands",
                [
                    ([(40, 140, 60)], "under ~6 h — a day trip to the sea"),
                    ([(190, 210, 70)], "~12 h"),
                    ([(235, 170, 50)], "~24 h"),
                    ([(200, 80, 40)], "~48 h"),
                    ([(120, 30, 80)], "~96 h — the deep interior"),
                ],
            ),
        ],
        [
            "Every water-derived time is conditional on the k_Q scale in "
            "force; k_Q sets how big rivers are, and therefore whether they "
            "speed travel as routes or slow it as obstacles.",
            "Sentinel cells (no land node) are not drawn.",
        ],
    )


if __name__ == "__main__":
    main()
