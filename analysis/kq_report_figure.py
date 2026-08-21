#!/usr/bin/env python3
"""Render the k_Q calibration report panel from a kq_calibration run.

Reads the JSON emitted by kq_calibration.py plus the terrain artifacts
of each world, and composes the three-panel figure that carries the
finding: the response curve and its two degeneracies, the effect of
the discharge-convention correction on channel size, and why the
reference worlds have no navigable water.

    uv run --project analysis python analysis/kq_report_figure.py \\
        <kq_calibration.json> --out analysis/out/kq_report.png
"""

import argparse
import json
import math
import struct
from array import array
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

DTYPE = {1: "i", 2: "I", 3: "Q", 4: "f"}
# Dossier 0015 §2: no published Qbf/Qmean exists; two independent
# derivations bracket it at 1.6-4.6 (median ~3).
QBF_OVER_QMEAN = 3.0
# W, D scale as Q^(3/8) through the Finnegan-Manning chain.
WIDTH_EXPONENT = 3.0 / 8.0


def read_vrast(path: Path):
    b = path.read_bytes()
    _, _, dtype, w, h, cell_cm = struct.unpack("<4sIIIII", b[:24])
    a = array(DTYPE[dtype])
    a.frombytes(b[24:])
    return w, h, cell_cm, a


def channel_slopes(run_dir: Path):
    w, h, cell_cm, strahler = read_vrast(run_dir / "strahler.vrast")
    _, _, _, recv = read_vrast(run_dir / "receivers.vrast")
    _, _, _, wd = read_vrast(run_dir / "water_depth.vrast")
    _, _, _, hcm = read_vrast(run_dir / "heights_cm.vrast")
    dx = cell_cm / 100.0
    out = []
    for i in range(len(strahler)):
        if strahler[i] == 0 or recv[i] == i or wd[i] != 0.0:
            continue
        r = recv[i]
        dz = (hcm[i] - hcm[r]) / 100.0
        d = dx * math.hypot((r % w) - (i % w), (r // w) - (i // w))
        if d > 0 and dz > 0:
            out.append(dz / d)
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("json_path", type=Path)
    ap.add_argument("--runs", nargs="*", type=Path, default=None)
    ap.add_argument("--out", type=Path, default=Path("analysis/out/kq_report.png"))
    ap.add_argument("--protocol", default="kq-cal-v1")
    ap.add_argument("--outlets", type=Path, default=None,
                    help="JSON [[area_km2, q_m3s], ...] of outlet points")
    ap.add_argument("--via-fit", nargs=2, type=float, default=None,
                    metavar=("A", "B"), help="fitted Q = A * area^B")
    args = ap.parse_args()

    rows = json.loads(args.json_path.read_text())
    runs = sorted({r["run"] for r in rows})
    kqs = sorted({r["k_q"] for r in rows})
    derived = [r["k_q"] for r in rows if r["is_derived"]]
    derived = sum(derived) / len(derived) if derived else None

    fig = plt.figure(figsize=(16.5, 5.6))
    fig.suptitle(
        f"k_Q calibration — {len(runs)}-seed ensemble, 512² at 200 m "
        f"(protocol {args.protocol}; every metric value is conditional on k_Q)",
        fontsize=12,
        y=0.98,
    )
    axes = [fig.add_subplot(1, 3, i + 1) for i in range(3)]

    # --- A: the response curve and its two degeneracies -------------
    ax = axes[0]
    for run in runs:
        pts = sorted(
            [(r["k_q"], 100 * r["fordable_frac"]) for r in rows if r["run"] == run]
        )
        ax.plot([p[0] for p in pts], [p[1] for p in pts], marker="o", ms=3, lw=1.2,
                alpha=0.85, label=run)
    if derived:
        ax.axvline(derived, color="crimson", ls="--", lw=1.6)
        ax.annotate(
            f"derived\n{derived:.2g}",
            xy=(derived, 52), xytext=(derived * 1.9, 60),
            fontsize=8.5, color="crimson",
            arrowprops=dict(arrowstyle="->", color="crimson", lw=1),
        )
    ax.axvline(1.0, color="dimgray", ls=":", lw=1.6)
    ax.annotate("shipped\nplaceholder 1.0", xy=(1.0, 30), xytext=(0.055, 34),
                fontsize=8.5, color="dimgray",
                arrowprops=dict(arrowstyle="->", color="dimgray", lw=1))
    ax.set_xscale("log")
    ax.set_xlabel("k_Q  (m³/s per discharge unit)")
    ax.set_ylabel("channel cells fordable  (%)")
    ax.set_title("A · Two degeneracies bracket the placeholder", fontsize=10)
    ax.grid(alpha=0.3, lw=0.5)
    ax.legend(fontsize=7, loc="lower left")

    # --- B: validation against real gauged rivers --------------------
    ax = axes[1]
    if args.outlets and args.outlets.exists():
        pts = json.loads(args.outlets.read_text())
        ax.scatter(
            [p[0] for p in pts], [p[1] for p in pts],
            s=7, color="#2c7fb8", alpha=0.45, zorder=3,
            label=f"via outlets, {len(pts)} across 5 worlds",
        )
    grid = [5.0 * (1.35**i) for i in range(20)]
    ax.plot(grid, [0.0214 * a**0.965 for a in grid], color="crimson", lw=2.0,
            zorder=4, label="438 USGS reference gauges\nQ = 0.0214·A$^{0.965}$")
    ax.fill_between(grid, [0.0214 * a**0.965 * 0.57 for a in grid],
                    [0.0214 * a**0.965 * 1.80 for a in grid],
                    color="crimson", alpha=0.10, zorder=1,
                    label="their p5–p95 spread")
    if args.via_fit:
        a0, b0 = args.via_fit
        ax.plot(grid, [a0 * a**b0 for a in grid], color="#1a1a1a", lw=1.6, ls="--",
                zorder=5, label=f"via fit: Q = {a0:.4f}·A$^{{{b0:.3f}}}$")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("drainage area at the outlet  (km²)")
    ax.set_ylabel("mean annual discharge  (m³/s)")
    ax.set_title("B · Calibrated discharge vs real gauged rivers", fontsize=10)
    ax.grid(alpha=0.3, lw=0.5, which="both")
    ax.legend(fontsize=7, loc="upper left")

    # --- C: why nothing is navigable ---------------------------------
    ax = axes[2]
    run_dirs = args.runs or [args.json_path.parent]
    plotted = False
    for rd in run_dirs:
        if not (rd / "strahler.vrast").exists():
            continue
        s = sorted(channel_slopes(rd))
        if not s:
            continue
        ys = [100 * i / len(s) for i in range(len(s))]
        ax.plot([100 * v for v in s], ys, lw=1.3, alpha=0.85, label=rd.name)
        plotted = True
    ax.axvline(3.0, color="crimson", ls="--", lw=1.6)
    ax.annotate(
        "≈3 %: Manning flow turns\nsupercritical (F>1), past the\nF≤0.9 limit of Langbein's Fig. 8",
        xy=(3.0, 32), xytext=(0.013, 52), fontsize=8, color="crimson",
        arrowprops=dict(arrowstyle="->", color="crimson", lw=1),
    )
    ax.set_xscale("log")
    ax.set_xlabel("local channel slope  (%)")
    ax.set_ylabel("channel cells below this slope  (%)")
    ax.set_title("C · These worlds are torrents, so nothing is navigable", fontsize=10)
    ax.grid(alpha=0.3, lw=0.5, which="both")
    if plotted:
        ax.legend(fontsize=7, loc="upper left")

    fig.tight_layout(rect=(0, 0.02, 1, 0.95))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(args.out, dpi=150)
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
