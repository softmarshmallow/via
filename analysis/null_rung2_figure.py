#!/usr/bin/env python3
"""Rung 2 of the ADR 0013 null ladder: generator vs suitability-only.

Same seed set, same origin-mass field. The only difference is the
spatial-interaction term, so any divergence is attributable to c_ij.
A third reference — K vertices drawn uniformly at random from the same
seed set — bounds how much of any spacing effect is just "K points on
this network".
"""
import json, struct, sys
from array import array
from collections import Counter
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from matplotlib.patches import FancyBboxPatch  # noqa: E402

DTYPE = {1: "i", 2: "I", 3: "Q", 4: "f"}
INK, RED, BLUE, GREEN, FLAG, GREY = "#1a1a1a", "#c1440e", "#2c7fb8", "#3f7d20", "#b8860b", "#8a8a8a"


def read_vrast(p):
    b = Path(p).read_bytes()
    _, _, dt, w, h, cc = struct.unpack("<4sIIIII", b[:24])
    a = array(DTYPE[dt]); a.frombytes(b[24:])
    return w, h, cc, np.array(a)


def nnd(xy, km):
    """Mean nearest-neighbour distance in km."""
    if len(xy) < 2:
        return float("nan")
    d = np.sqrt(((xy[:, None, :] - xy[None, :, :]) ** 2).sum(-1))
    np.fill_diagonal(d, np.inf)
    return float(d.min(axis=1).mean() * km)


def main(run, out):
    run = Path(run)
    s = json.load(open(run / "settlement.site.json"))
    gen = s["settlements"]
    nul = s["null_rung2_suitability_only"]
    w, h, cc, heights = read_vrast(run / "heights_cm.vrast")
    _, _, _, wd = read_vrast(run / "water_depth.vrast")
    km = cc / 100.0 / 1000.0

    gs = np.array([x["share"] for x in gen]); go = np.argsort(-gs)
    ns = np.array([x["share"] for x in nul]); no = np.argsort(-ns)
    ls = np.log10(gs[go]); K = int(np.argmax(ls[:-1] - ls[1:])) + 1

    gtop = [gen[i] for i in go[:K]]
    ntop = [nul[i] for i in no[:K]]
    gcells = {x["cell"] for x in gtop}
    ncells = {x["cell"] for x in ntop}
    overlap = len(gcells & ncells)

    gxy = np.array([[x["x"], x["y"]] for x in gtop], float)
    nxy = np.array([[x["x"], x["y"]] for x in ntop], float)
    allxy = np.array([[x["x"], x["y"]] for x in gen], float)
    rng = np.random.default_rng(0)
    rand_nnd = [nnd(allxy[rng.choice(len(allxy), K, replace=False)], km) for _ in range(200)]
    rnd_lo, rnd_hi = np.percentile(rand_nnd, [2.5, 97.5])
    g_nnd, n_nnd = nnd(gxy, km), nnd(nxy, km)

    fig = plt.figure(figsize=(16.2, 10.4), dpi=150)
    fig.patch.set_facecolor("white")
    fig.text(0.028, 0.975, "Rung 2 of the null ladder — does the interaction term earn its keep?",
             fontsize=22, color=INK, fontweight="bold", va="top")
    fig.text(0.028, 0.944,
             f"runs/big-s102 · same {len(gen)} seeds, same origin-mass field · the null drops only the spatial-interaction term · "
             f"top-{K} sets compared",
             fontsize=10, color="#555", va="top")
    fig.text(0.028, 0.918,
             f"They pick almost different worlds: only {overlap} of {K} centres are shared "
             f"({100*overlap/K:.0f}%).",
             fontsize=11.6, color=RED, fontweight="bold", va="top")

    land = (wd.reshape(h, w) <= 0)
    elev = np.where(land, np.clip(heights.reshape(h, w) / 100.0, 0, None), np.nan)

    for idx, (title, top, col) in enumerate([
            (f"A · Generator (Harris–Wilson) — top {K}", gtop, RED),
            (f"B · Null rung 2 (suitability only) — top {K}", ntop, BLUE)]):
        ax = fig.add_axes([0.026 + idx * 0.290, 0.075, 0.278, 0.790])
        ax.imshow(np.where(land, 1.0, 0.0), cmap="Greys", vmin=0, vmax=6,
                  origin="upper", interpolation="nearest")
        ax.imshow(elev, cmap="terrain", alpha=0.5, origin="upper", interpolation="nearest")
        sh = np.array([x["share"] for x in top])
        xx = np.array([x["x"] for x in top]); yy = np.array([x["y"] for x in top])
        shared = np.array([x["cell"] in (gcells & ncells) for x in top])
        size = 20 + 900 * (sh / sh.max()) ** 0.8
        ax.scatter(xx[~shared], yy[~shared], s=size[~shared], c=col, alpha=0.85,
                   linewidths=0.6, edgecolors="white", zorder=4)
        ax.scatter(xx[shared], yy[shared], s=size[shared], facecolor="none",
                   edgecolors=INK, linewidths=1.8, zorder=5)
        ax.set_xlim(0, w); ax.set_ylim(h, 0); ax.set_xticks([]); ax.set_yticks([])
        ax.set_title(title, fontsize=10.8, color=INK, fontweight="bold")
        ax.text(0.02, 0.02, "black ring = chosen by both", transform=ax.transAxes,
                fontsize=8, color=INK,
                bbox=dict(boxstyle="round,pad=0.3", fc="white", ec="#ccc", alpha=0.9))

    # C: class composition
    axc = fig.add_axes([0.628, 0.600, 0.165, 0.268])
    keys = ["junction", "river_mouth", "pass", "head_of_navigation"]
    labs = ["junction\nonly", "river\nmouth", "pass", "head of\nnav."]
    gc = Counter("junction" if x["class"] is None else x["class"] for x in gtop)
    ncnt = Counter("junction" if x["class"] is None else x["class"] for x in ntop)
    yy = np.arange(len(keys))
    axc.barh(yy - 0.19, [gc.get(k, 0) for k in keys], height=0.36, color=RED, label="generator")
    axc.barh(yy + 0.19, [ncnt.get(k, 0) for k in keys], height=0.36, color=BLUE, label="null")
    for i, k in enumerate(keys):
        axc.text(gc.get(k, 0) + 0.4, yy[i] - 0.19, str(gc.get(k, 0)), fontsize=8,
                 va="center", color=RED, fontweight="bold")
        axc.text(ncnt.get(k, 0) + 0.4, yy[i] + 0.19, str(ncnt.get(k, 0)), fontsize=8,
                 va="center", color=BLUE, fontweight="bold")
    axc.set_yticks(yy); axc.set_yticklabels(labs, fontsize=8.2); axc.invert_yaxis()
    axc.set_xlabel(f"count of the top {K}", fontsize=9)
    axc.set_title("C · What each picks", fontsize=10.6, color=INK, fontweight="bold")
    axc.tick_params(labelsize=8); axc.grid(axis="x", alpha=0.16, lw=0.6)
    for sp in ("top", "right", "left"):
        axc.spines[sp].set_visible(False)
    axc.legend(fontsize=8, loc="lower right", framealpha=0.93)

    # D: spacing
    axd = fig.add_axes([0.838, 0.600, 0.142, 0.268])
    axd.hist(rand_nnd, bins=24, color="#ddd", edgecolor="#bbb")
    axd.axvline(g_nnd, color=RED, lw=2.2)
    axd.axvline(n_nnd, color=BLUE, lw=2.2, ls="--")
    axd.set_xlabel("mean NN distance (km)", fontsize=9)
    axd.set_ylabel("random draws", fontsize=9)
    axd.set_title("D · Spacing vs random\nK drawn from the same seeds",
                  fontsize=10.6, color=INK, fontweight="bold", linespacing=1.4)
    axd.tick_params(labelsize=8)
    for sp in ("top", "right"):
        axd.spines[sp].set_visible(False)

    def note(x, y, ww, hh, edge, fill, title, body, tcol=None):
        fig.patches.append(FancyBboxPatch((x, y), ww, hh,
                           boxstyle="round,pad=0,rounding_size=0.008",
                           linewidth=1.6, edgecolor=edge, facecolor=fill,
                           transform=fig.transFigure, zorder=1))
        fig.text(x + 0.012, y + hh - 0.020, title, fontsize=10.3,
                 color=tcol or edge, fontweight="bold", va="top")
        fig.text(x + 0.012, y + hh - 0.046, body, fontsize=8.3, color="#333",
                 va="top", linespacing=1.52)

    verdict = ("The interaction term is doing real work" if overlap < K * 0.5
               else "The interaction term adds little here")
    note(0.628, 0.290, 0.352, 0.240, GREEN if overlap < K * 0.5 else FLAG,
         "#f1f7ee" if overlap < K * 0.5 else "#fdf4dd",
         verdict,
         f"Shared centres: {overlap} of {K}. The generator takes\n"
         f"{gc.get('junction',0)} junction-only vertices; the null takes {ncnt.get('junction',0)}.\n\n"
         f"Mean nearest-neighbour distance\n"
         f"  generator {g_nnd:.1f} km\n"
         f"  null      {n_nnd:.1f} km\n"
         f"  random K  {np.mean(rand_nnd):.1f} km  (95% {rnd_lo:.1f}–{rnd_hi:.1f})\n\n"
         "O_i is identical for both, so every difference above is\n"
         "attributable to c_ij and nothing else.")

    note(0.628, 0.075, 0.352, 0.200, FLAG, "#fdf4dd",
         "This is one seed, and not yet the D9 gate",
         "ADR 0008 D5: single-seed numbers are never evidence. A four-\n"
         "seed ensemble is generating now; until it lands these are a\n"
         "single observation, not a distribution.\n\n"
         "D9 also asks for a character the null demonstrably FAILS,\n"
         "measured with global envelopes at M = 19. Set overlap and mean\n"
         "NN distance are diagnostics pointing that way, not that test.",
         tcol="#8a6508")

    fig.savefig(out, facecolor="white")
    print(out)
    print(f"K={K} overlap={overlap} g_nnd={g_nnd:.2f} n_nnd={n_nnd:.2f} "
          f"rand={np.mean(rand_nnd):.2f} [{rnd_lo:.2f},{rnd_hi:.2f}]")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
