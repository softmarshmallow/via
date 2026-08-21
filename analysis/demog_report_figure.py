#!/usr/bin/env python3
"""Report panel for the re-run demographic targets.

Every number plotted here was read by hand from a PDF opened in this
session and cross-checked against `pdftotext -layout` output. Sources
and page numbers are printed on the panel. Nothing is via output.
"""

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.patches import FancyBboxPatch  # noqa: E402

INK = "#1a1a1a"
BLUE = "#2c7fb8"
RED = "#c1440e"
FLAG = "#b8860b"

# Jedwab & Vollrath (2019) AEJ:Macro 11(1), Figure 3, p. 232.
# Unweighted means over 392 city-period observations, per 1,000 people.
ERAS = [
    ("pre-1800s", 38, 38.1, 36.1, 2.0),
    ("1820s–50s", 33, 35.6, 30.4, 5.2),
    ("1880s", 69, 34.0, 28.0, 5.9),
    ("1900s", 89, 28.0, 22.1, 5.9),
    ("1960s", 63, 33.2, 10.9, 22.3),
    ("2000s", 100, 17.4, 6.5, 11.0),
]

# JJK, IIEP-WP-2020-14, Table 1, p. 48. mortality %, low, high.
# None = no low/high estimate printed.
PLAGUE = [
    ("England & Scotland", 6.0, 55.0, 45.0, 62.5),
    ("Scandinavia", 1.9, 55.0, 50.0, 60.0),
    ("France", 16.0, 50.0, 30.0, 60.0),
    ("Italy", 12.5, 50.0, 40.0, 55.0),
    ("Spain", 5.5, 50.0, 30.0, 62.5),
    ("Netherlands", 0.8, 32.5, 30.0, 35.0),
    ("Poland", 2.0, 25.0, None, None),
    ("Belgium", 1.4, 22.5, 20.0, 25.0),
    ("Germany", 13.0, 22.5, 20.0, 25.0),
    ("Austria/Czechia/Hungary", 10.0, 20.0, 15.0, None),
]
WEUR_POP, WEUR_MORT, LOC274 = 72.8, 38.75, 38.90


def main(out):
    fig = plt.figure(figsize=(15.0, 11.2), dpi=150)
    fig.patch.set_facecolor("white")

    fig.text(0.035, 0.965, "The two retracted targets, re-run directly",
             fontsize=22, color=INK, fontweight="bold", va="top")
    fig.text(0.035, 0.933,
             "Both PDFs opened and read in-session, then cross-checked against pdftotext output. "
             "Page numbers are printed beside every figure.",
             fontsize=10, color="#555", va="top")

    # ---------- Panel A: the 45-degree line ---------------------------
    axa = fig.add_axes([0.055, 0.500, 0.335, 0.375])
    lim = (0, 42)
    axa.plot(lim, lim, color="#999", lw=1.2, zorder=1)
    axa.text(37, 38.6, "CBR = CDR\n(no natural increase)", fontsize=7.6,
             color="#777", ha="right", va="bottom", linespacing=1.3)
    for i, (name, n, cbr, cdr, crni) in enumerate(ERAS):
        col = RED if i == 0 else (BLUE if i < 4 else "#3f7d20")
        axa.scatter([cdr], [cbr], s=95, color=col, zorder=3,
                    edgecolor="white", linewidth=1.2)
        dx, dy = (1.6, 1.2)
        if name == "pre-1800s":
            dx, dy = 1.6, -3.4
        elif name == "1880s":
            dx, dy = -1.8, -3.0
        elif name == "1900s":
            dx, dy = -1.4, 1.8
        axa.annotate(name, xy=(cdr, cbr), xytext=(cdr + dx, cbr + dy),
                     fontsize=8.6, color=col, fontweight="bold",
                     ha="right" if dx < 0 else "left")
    axa.set_xlim(*lim)
    axa.set_ylim(*lim)
    axa.set_xlabel("crude death rate (per 1,000)", fontsize=9)
    axa.set_ylabel("crude birth rate (per 1,000)", fontsize=9)
    axa.set_title("A · The world's largest cities sat on the line\n"
                  "Jedwab & Vollrath 2019, Fig. 3 means, p. 232",
                  fontsize=10.5, color=INK, fontweight="bold", linespacing=1.4)
    axa.tick_params(labelsize=8)
    axa.grid(alpha=0.18, lw=0.7)
    for s in ("top", "right"):
        axa.spines[s].set_visible(False)

    # ---------- Panel B: CRNI bars ------------------------------------
    axb = fig.add_axes([0.455, 0.500, 0.245, 0.375])
    names = [e[0] for e in ERAS]
    crnis = [e[4] for e in ERAS]
    cols = [RED] + [BLUE] * 3 + ["#3f7d20"] * 2
    axb.barh(range(len(names)), crnis, color=cols, height=0.62)
    for i, v in enumerate(crnis):
        axb.text(v + 0.6, i, f"{v}", fontsize=8.6, va="center",
                 color=cols[i], fontweight="bold")
    axb.set_yticks(range(len(names)))
    axb.set_yticklabels(names, fontsize=8.6)
    axb.invert_yaxis()
    axb.set_xlim(0, 26)
    axb.set_xlabel("natural increase (per 1,000/yr)", fontsize=9)
    axb.set_title("B · Urban natural increase\nnormal periods only (fn. 9, p. 233)",
                  fontsize=10.5, color=INK, fontweight="bold", linespacing=1.4)
    axb.tick_params(labelsize=8)
    axb.grid(axis="x", alpha=0.18, lw=0.7)
    for s in ("top", "right", "left"):
        axb.spines[s].set_visible(False)

    # ---------- Panel C: plague mortality -----------------------------
    axc = fig.add_axes([0.775, 0.500, 0.190, 0.375])
    ys = range(len(PLAGUE))
    for i, (name, pop, m, lo, hi) in enumerate(PLAGUE):
        if lo is not None and hi is not None:
            axc.plot([lo, hi], [i, i], color="#bbb", lw=3.0, solid_capstyle="round",
                     zorder=1)
        axc.scatter([m], [i], s=18 + pop * 4.5, color=RED, alpha=0.85,
                    zorder=3, edgecolor="white", linewidth=1.0)
    axc.axvline(WEUR_MORT, color=INK, ls="--", lw=1.4, zorder=2)
    axc.text(WEUR_MORT - 1.4, -1.05,
             f"W. Europe {WEUR_MORT}%\n72.8 m in 1300",
             fontsize=7.8, color=INK, va="center", ha="right",
             linespacing=1.35)
    axc.set_yticks(list(ys))
    axc.set_yticklabels([p[0] for p in PLAGUE], fontsize=8.0)
    axc.set_ylim(len(PLAGUE) - 0.4, -1.7)
    axc.set_xlim(10, 68)
    axc.set_xlabel("Black Death mortality (%)", fontsize=9)
    axc.set_title("C · The shock magnitude\nJJK IIEP-WP-2020-14, Tab. 1, p. 48",
                  fontsize=10.5, color=INK, fontweight="bold", linespacing=1.4)
    axc.tick_params(labelsize=8)
    axc.grid(axis="x", alpha=0.18, lw=0.7)
    for s in ("top", "right", "left"):
        axc.spines[s].set_visible(False)
    axc.text(0.03, 0.035, "marker area ∝ 1300 population",
             transform=axc.transAxes, fontsize=7.2, color="#888", ha="left")

    # ---------- the findings ------------------------------------------
    def note(x, y, w, h, edge, fill, title, body, tcol=None):
        fig.patches.append(FancyBboxPatch(
            (x, y), w, h, boxstyle="round,pad=0,rounding_size=0.008",
            linewidth=1.6, edgecolor=edge, facecolor=fill,
            transform=fig.transFigure, zorder=1))
        fig.text(x + 0.013, y + h - 0.018, title, fontsize=10.5,
                 color=tcol or edge, fontweight="bold", va="top")
        fig.text(x + 0.013, y + h - 0.045, body, fontsize=8.7,
                 color="#333", va="top", linespacing=1.55)

    note(0.035, 0.258, 0.455, 0.198, RED, "#fbe6dd",
         "The finding that changes the stage",
         "The sign of pre-industrial urban natural increase is contested — and the\n"
         "disagreement is the mechanism. JJK (WP p. 23) say it was \"typically negative\n"
         "in urban areas until the 19th century\", citing the very paper that measures\n"
         "+2.0 per 1,000. JV's footnote 9 (p. 233) reconciles them: panel A is \"normal\"\n"
         "periods only, and \"during the Black Death, cities had death rates of 250–750\".\n\n"
         "So there is no single rate. There is a normal-period rate of ≈ +2 per 1,000/yr\n"
         "and a shock process. Force one mean and you either lose the shocks or\n"
         "double-count them.")

    note(0.510, 0.258, 0.455, 0.198, BLUE, "#dceaf4",
         "Three constraints the shock record imposes",
         "1 · The plague did not discriminate by settlement type — \"similar death rates\n"
         "     were recorded on average in urban and in rural areas\" (WP p. 21).\n"
         "2 · Nor by site quality: for 165 cities, mortality was \"uncorrelated with\n"
         "     various city characteristics proxying for physical geography, economic\n"
         "     geography, human capital and institutions\" (WP p. 49). A shock in via\n"
         "     must therefore NOT be conditioned on the suitability field.\n"
         "3 · Recovery is path-dependent: cities recovered on average by 1500, but\n"
         "     Barcelona/Florence/Lübeck/Venice took 5/30/10/25 years while Narbonne\n"
         "     and Winchester \"shrank to insignificance\" (WP pp. 21–22).")

    note(0.035, 0.042, 0.930, 0.192, FLAG, "#fdf4dd",
         "What is still missing — stated as a gap, not filled",
         "The aggregate pre-industrial population growth series — the actual forcing quantity the settlement stage needs — is in neither paper.\n"
         "It requires the Maddison Project, McEvedy & Jones (1978), or Broadberry et al., none of which have been opened. Also unread: the published\n"
         "JEL 60(1):132–178 (paywalled — every JJK page cite here is to the open working paper and must never be attributed to the journal), Wrigley\n"
         "(1967), Woods (2003), and any Bairoch/de Vries transcription. The pre-1800 RURAL rate is plotted in JV Figure 4 panel C but stated nowhere in\n"
         "the text, so it is deliberately not recorded — obtaining it would mean reading a value off a scatter panel.\n\n"
         "Consequence for ADR 0013: the population budget stays Tier-2 forcing. Four mechanism constraints now have citations; its magnitude still does not.",
         tcol="#8a6508")

    fig.text(0.035, 0.012,
             "Sources read in session: Jedwab & Vollrath (2019) AEJ: Macroeconomics 11(1):223–275, published PDF · "
             "Jedwab, Johnson & Koyama, IIEP-WP-2020-14, 4 August 2020.",
             fontsize=8.0, color="#777", va="bottom", style="italic")

    fig.savefig(out, facecolor="white")
    print(out)


if __name__ == "__main__":
    import sys

    main(sys.argv[1] if len(sys.argv) > 1 else "demog_panel.png")
