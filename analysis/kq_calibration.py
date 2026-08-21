#!/usr/bin/env python3
"""Measure the affordance chain's response to the k_Q discharge scale.

k_Q converts via's relative discharge (mean-precipitation-equivalent
upslope cells) into m³/s. Everything metric downstream of it — channel
width, depth, velocity, crossability, navigability — inherits it, so a
declared value needs a measured response curve behind it, not a guess.

This is an instrument, not a tuner: it runs the real via-suitability
stage once per (run, k_Q) pair under a namespaced config label and
reads back the rasters the stage emitted, so what is plotted is what
the stage computes — no hydraulics are reimplemented here. Quarantined
sidecar (ADR 0009 D4): never imported by a crate, no gate depends on it.

Usage:

    uv run --project analysis python analysis/kq_calibration.py \\
        runs/m4c-research-s42 runs/kq-ens-s43 ... \\
        --kq 1e-4 3e-4 1e-3 3e-3 1e-2 --out analysis/out/kq.png
"""

import argparse
import json
import struct
import subprocess
import sys
from array import array
from pathlib import Path

DTYPE = {1: "i", 2: "I", 3: "Q", 4: "f"}
SECONDS_PER_YEAR = 365.25 * 24 * 3600


def read_vrast(path: Path):
    b = path.read_bytes()
    magic, ver, dtype, w, h, cell_cm = struct.unpack("<4sIIIII", b[:24])
    if magic != b"VRAS":
        raise ValueError(f"{path}: not a .vrast")
    a = array(DTYPE[dtype])
    a.frombytes(b[24:])
    return w, h, cell_cm, a


def terrain_config(run_dir: Path) -> dict:
    manifest = json.loads((run_dir / "manifest.json").read_text())
    return manifest["stages"]["terrain"]["config"]


def derived_kq(run_dir: Path, runoff_ratio: float) -> float:
    """k_Q implied by the grid's own hydrology.

    via's discharge is a count of mean-precipitation-equivalent cells,
    so one unit carries A_cell x P_mean of precipitation per year, of
    which the runoff ratio reaches the channel.
    """
    cfg = terrain_config(run_dir)
    a_cell = cfg["cell_size_m"] ** 2
    return a_cell * cfg["precip_mean_m_per_yr"] * runoff_ratio / SECONDS_PER_YEAR


def run_suitability(run_dir: Path, label: str, kq: float, via_bin: Path) -> None:
    """Run the real stage under a namespaced label."""
    cfg = {"label": label, "k_q_m3s_per_unit": kq}
    cfg_path = run_dir / f"_kq_{label}.json"
    cfg_path.write_text(json.dumps(cfg))
    proc = subprocess.run(
        [str(via_bin), "suitability", str(run_dir), "--config", str(cfg_path)],
        capture_output=True,
        text=True,
    )
    cfg_path.unlink(missing_ok=True)
    if proc.returncode != 0:
        raise RuntimeError(f"{run_dir} k_Q={kq:g}: {proc.stderr.strip()[-500:]}")


def measure(run_dir: Path, label: str, caps: dict) -> dict:
    """Read back what the stage emitted and summarise the channels."""
    def raster(name):
        return read_vrast(run_dir / f"suitability.{label}.{name}.vrast")[3]

    _, _, cell_cm, strahler = read_vrast(run_dir / "strahler.vrast")
    _, _, _, receivers = read_vrast(run_dir / "receivers.vrast")
    _, _, _, depth_std = read_vrast(run_dir / "water_depth.vrast")
    width = raster("ford_width")
    depth = raster("ford_depth")
    vel = raster("ford_velocity")
    dv = raster("crossability")
    navigable = raster("navigable")
    _, _, _, area = read_vrast(run_dir / "area_cells.vrast")

    chan = [
        i
        for i in range(len(strahler))
        if strahler[i] > 0 and receivers[i] != i and depth_std[i] == 0.0
    ]
    if not chan:
        raise RuntimeError(f"{run_dir}: no channel cells")

    def pct(vals, q):
        s = sorted(vals)
        return s[min(len(s) - 1, int(len(s) * q))]

    fordable = sum(
        1
        for i in chan
        if dv[i] <= caps["dv"] and depth[i] <= caps["depth"] and vel[i] <= caps["vel"]
    )
    nav = sum(1 for i in chan if navigable[i] == 1)
    outlet = max(chan, key=lambda i: area[i])
    a_cell_km2 = (cell_cm / 100.0) ** 2 / 1e6
    return {
        "channel_cells": len(chan),
        "fordable_frac": fordable / len(chan),
        "navigable_frac": nav / len(chan),
        "width_median_m": pct([width[i] for i in chan], 0.5),
        "depth_median_m": pct([depth[i] for i in chan], 0.5),
        "vel_median_ms": pct([vel[i] for i in chan], 0.5),
        "outlet_area_km2": area[outlet] * a_cell_km2,
        "outlet_width_m": width[outlet],
        "outlet_depth_m": depth[outlet],
        "outlet_vel_ms": vel[outlet],
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dirs", nargs="+", type=Path)
    ap.add_argument(
        "--kq",
        nargs="+",
        type=float,
        required=True,
        help="candidate k_Q values (m3/s per discharge unit)",
    )
    ap.add_argument(
        "--runoff-ratio",
        type=float,
        default=None,
        help="if given, also measure the k_Q derived from this runoff ratio",
    )
    ap.add_argument("--via-bin", type=Path, default=Path("target/release/via"))
    ap.add_argument("--out", type=Path, default=Path("analysis/out/kq_calibration.png"))
    ap.add_argument("--json-out", type=Path, default=None)
    ap.add_argument(
        "--caps",
        nargs=3,
        type=float,
        default=[0.8, 1.2, 3.0],
        metavar=("DV", "DEPTH", "VEL"),
        help="Cox/AIDR ford caps used for the fordable fraction",
    )
    args = ap.parse_args()
    caps = {"dv": args.caps[0], "depth": args.caps[1], "vel": args.caps[2]}

    rows = []
    for run_dir in args.run_dirs:
        kqs = list(args.kq)
        derived = None
        if args.runoff_ratio is not None:
            derived = derived_kq(run_dir, args.runoff_ratio)
            kqs.append(derived)
        for n, kq in enumerate(sorted(set(kqs))):
            label = f"kq{n:02d}"
            print(f"  {run_dir.name}  k_Q={kq:.4g} ...", file=sys.stderr, flush=True)
            run_suitability(run_dir, label, kq, args.via_bin)
            m = measure(run_dir, label, caps)
            m.update(
                run=run_dir.name,
                k_q=kq,
                is_derived=(derived is not None and kq == derived),
            )
            rows.append(m)

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(rows, indent=2) + "\n")
        print(f"wrote {args.json_out}")

    plot(rows, args)
    return 0


def plot(rows, args) -> None:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    runs = sorted({r["run"] for r in rows})
    fig, axes = plt.subplots(1, 3, figsize=(15, 4.6))
    fig.suptitle(
        "k_Q response of the affordance chain "
        f"({len(runs)}-seed ensemble; Cox caps D·V≤{args.caps[0]:g}, "
        f"d≤{args.caps[1]:g} m, v≤{args.caps[2]:g} m/s)",
        fontsize=11,
    )

    panels = [
        ("fordable_frac", "fraction of channel cells fordable", axes[0]),
        ("navigable_frac", "fraction of channel cells navigable", axes[1]),
        ("outlet_width_m", "width at the largest outlet (m)", axes[2]),
    ]
    for key, ylabel, ax in panels:
        for run in runs:
            pts = sorted(
                [(r["k_q"], r[key]) for r in rows if r["run"] == run], key=lambda p: p[0]
            )
            ax.plot(
                [p[0] for p in pts],
                [p[1] for p in pts],
                marker="o",
                ms=3,
                lw=1.2,
                alpha=0.85,
                label=run,
            )
        derived = [r["k_q"] for r in rows if r["is_derived"]]
        if derived:
            ax.axvline(
                sum(derived) / len(derived),
                color="crimson",
                ls="--",
                lw=1.4,
                label="derived k_Q",
            )
        ax.set_xscale("log")
        if key == "outlet_width_m":
            ax.set_yscale("log")
        ax.set_xlabel("k_Q  (m³/s per discharge unit)")
        ax.set_ylabel(ylabel)
        ax.grid(alpha=0.3, lw=0.5)
    axes[0].legend(fontsize=7, loc="best")
    fig.tight_layout()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(args.out, dpi=140)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    raise SystemExit(main())
