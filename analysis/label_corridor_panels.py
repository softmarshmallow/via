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
            f"k_Q degeneracy, declared: {ford_pct:.1f}% of channel cells are "
            f"fordable and {nav_pct:.1f}% navigable at the k_Q placeholder, so "
            "rivers act as walls — cross-river movement detours via headwater "
            "ridges or the coast. That shape is the declared consequence of "
            "one uncalibrated forcing constant, not terrain truth.",
            "Density shows only the top 20% of traversed cells (White & "
            "Barber's 80/20 rendering default); the raster ships raw.",
            f"reuse discount alpha = {cfg['reuse_alpha']:g}; ford caps "
            f"D*V <= {cfg['ford_max_dv_m2s']:g} m^2/s, depth <= "
            f"{cfg['ford_max_depth_m']:g} m, velocity <= "
            f"{cfg['ford_max_velocity_ms']:g} m/s (Cox/AIDR).",
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
            "Every water-derived time is conditional on the declared k_Q "
            "scale; at the placeholder the rivers are impassable walls, "
            "which stretches interior times.",
            "Sentinel cells (no land node) are not drawn.",
        ],
    )


if __name__ == "__main__":
    main()
