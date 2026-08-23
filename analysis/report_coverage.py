#!/usr/bin/env python3
"""Coverage verification for the Benchmark Report (0012 §9.1; v1.1).

The pre-registration recorded an acquisition-time building-coverage
expectation per town (`community-mapped`, `expected-good`,
`unverified`, or the pilots' measured dash). Measurement applied the
protocol's 30-footprint floor mechanically. This script confronts the
two: per coverage flag, how many towns cleared the floor and what
footprint counts the disc actually held — and names every town whose
building characters are suppressed, plus every town whose flag
*promised* coverage the disc did not deliver (the surprising cases the
report must discuss).

Output is markdown on stdout: aggregate tables plus flag lists — no
corpus-scale character table (ADR 0010 Decision 5; footprint counts
for named below-floor towns are coverage flags, not a character
database).

Usage: uv run --project analysis python analysis/report_coverage.py \
        [--measured runs/reference/corpus-v1.1]
"""

import argparse
import json
import math
import sys

from fetch_corpus import CLASSES, parse_sample
from fetch_extract import PRE_REGISTRATION, repo_path

INFORMAL_CLASS = "contemporary informal"


def nearest_rank(values: list[float], q: float) -> float:
    """The protocol's pinned quantile convention (v1.1 Elements)."""
    vals = sorted(values)
    return vals[int(math.floor((len(vals) - 1) * q + 0.5))]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--measured", default="runs/reference/corpus-v1.1")
    args = ap.parse_args()

    towns = parse_sample(repo_path(PRE_REGISTRATION).read_text(encoding="utf-8"))
    measured = repo_path(args.measured)

    # D6: published coverage OUTCOMES cover the fitted partition only.
    # Held-out towns' registration-time flags are already public in the
    # pre-registration document, but their measured footprint counts and
    # floor outcomes stay in the store and appear here only as a count.
    held_out = [t for t in towns if t["partition"] == "held-out"]
    towns = [t for t in towns if t["partition"] == "fitted"]

    rows = []
    for t in towns:
        path = measured / f"{t['town']}-fabric.json"
        if not path.exists():
            print(f"warning: {t['town']} unmeasured, skipped", file=sys.stderr)
            continue
        doc = json.loads(path.read_text(encoding="utf-8"))
        operative = "all_ways" if t["town_class"] == INFORMAL_CLASS else "carriageway"
        fab = doc["fabric"][operative]
        flag = t["coverage"]
        if flag.startswith("—") or "pilot" in flag:
            flag = "pilot (measured)"
        rows.append(
            {
                "town": t["town"],
                "class": t["town_class"],
                "flag": flag,
                "buildings": fab["buildings"],
                "below": fab["below_footprint_floor"],
            }
        )

    print("### Coverage flags vs the measured floor (fitted partition)\n")
    print("| coverage flag | towns | below floor | footprints median | min |")
    print("| --- | --- | --- | --- | --- |")
    for flag in sorted({r["flag"] for r in rows}):
        grp = [r for r in rows if r["flag"] == flag]
        counts = [r["buildings"] for r in grp]
        print(
            f"| {flag} | {len(grp)} | {sum(1 for r in grp if r['below'])} | "
            f"{nearest_rank(counts, 0.5):.0f} | {min(counts)} |"
        )
    ho_below = 0
    for t in held_out:
        path = measured / f"{t['town']}-fabric.json"
        if path.exists():
            doc = json.loads(path.read_text(encoding="utf-8"))
            op = "all_ways" if t["town_class"] == INFORMAL_CLASS else "carriageway"
            ho_below += bool(doc["fabric"][op]["below_footprint_floor"])
    print(
        f"\nHeld-out towns: {len(held_out)} measured; floor outcomes recorded "
        f"in the store, reported here only as a count — {ho_below} below floor."
    )

    below = [r for r in rows if r["below"]]
    print("\n### Towns below the 30-footprint floor (building characters suppressed)\n")
    if not below:
        print("None.")
    else:
        print("| town | class | coverage flag | footprints in disc |")
        print("| --- | --- | --- | --- |")
        for r in sorted(below, key=lambda r: (r["class"], r["town"])):
            print(f"| {r['town']} | {r['class']} | {r['flag']} | {r['buildings']} |")

    surprising = [
        r
        for r in rows
        if r["below"] and ("expected-good" in r["flag"] or "community-mapped" in r["flag"])
    ]
    print("\n### Flag contradictions (coverage promised, floor tripped)\n")
    if not surprising:
        print("None.")
    else:
        for r in surprising:
            print(f"- **{r['town']}** ({r['class']}): flagged `{r['flag']}`, "
                  f"disc held {r['buildings']} footprints")

    per_class = {c: [r for r in rows if r["class"] == c] for c, *_ in [(c,) for c in CLASSES]}
    print("\n### Floor outcomes per class\n")
    print("| class | measured | below floor |")
    print("| --- | --- | --- |")
    for c in CLASSES:
        grp = per_class[c]
        print(f"| {c} | {len(grp)} | {sum(1 for r in grp if r['below'])} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())
