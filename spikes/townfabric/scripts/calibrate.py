#!/usr/bin/env python3
"""Calibrate the growth parameters against a measured reference town.

A throwaway driver (CONTRIBUTING allows Python only as a script run
*against* exported artifacts — it drives the binary and reads its JSON).

Calibration is not validation: parameters are fitted here on ONE town, and
must then be checked against a town that was not used for fitting.
"""
import json, subprocess, sys, itertools, copy, os

BIN = "./target/release/via-spike-townfabric"
CFG = "spikes/townfabric/eras/three-fabrics.json"
OUT = "/private/tmp/claude-501/-Users-universe-Documents-badromancestudio-via/29dcefda-cfc5-4612-bc05-bb612d6cb0e0/scratchpad/sweep"
RUN = "runs/m4c-research-s42"
SPIKE = "runs/spike-settlements-s42/spike.json"

# Statistics the growth parameters actually control, and the weight each
# carries. Setback/wall are excluded: those are plot parameters, already fixed.
KEYS = [
    ("blocks", 1.0),
    ("block_area_median_m2", 1.0),
    ("block_area_p90_m2", 1.0),
    ("meshedness", 1.5),
    ("dead_end_share", 1.5),
    ("segment_len_median_m", 1.0),
]

def score(fab, ref):
    """Mean weighted relative error, in log space for the heavy-tailed areas."""
    import math
    tot = w_tot = 0.0
    for k, w in KEYS:
        a, b = fab.get(k), ref.get(k)
        if a is None or b is None:
            continue
        if "area" in k or k == "blocks" or "len" in k:
            a, b = max(a, 1e-6), max(b, 1e-6)
            e = abs(math.log(a / b))
        else:
            e = abs(a - b) / max(abs(b), 0.05)
        tot += w * e
        w_tot += w
    return tot / max(w_tot, 1e-9)

def run(town_name, overrides):
    base = json.load(open(CFG))
    town = [t for t in base["towns"] if t["name"] == town_name][0]
    town = copy.deepcopy(town)
    for k, v in overrides.items():
        town["growth"][k] = v
    base["towns"] = [town]
    os.makedirs(OUT, exist_ok=True)
    tmp = os.path.join(OUT, "cfg.json")
    json.dump(base, open(tmp, "w"))
    subprocess.run([BIN, "grow", RUN, "--config", tmp, "--out", OUT,
                    "--from-spike", SPIKE, "--era", "medieval", "--rank", "0"],
                   check=True, capture_output=True)
    rec = json.load(open(os.path.join(OUT, "townfabric.json")))
    # Score on the streets-only measurement: that is the protocol the
    # reference towns are measured under, and comparing a synthetic graph
    # that includes lanes against a real one that excludes them would be
    # scoring the protocol rather than the town.
    return rec["towns"][0]["fabric_streets_only"]

if __name__ == "__main__":
    town = sys.argv[1] if len(sys.argv) > 1 else "medieval organic"
    ref_name = sys.argv[2] if len(sys.argv) > 2 else "alnwick"
    ref = json.load(open(f"runs/reference/{ref_name}-fabric.json"))["fabric"]
    print(f"reference {ref_name}: blocks {ref['blocks']}  blk med {ref['block_area_median_m2']:.0f}  "
          f"p90 {ref['block_area_p90_m2']:.0f}  M {ref['meshedness']:.3f}  "
          f"dead {ref['dead_end_share']:.2f}  seg {ref['segment_len_median_m']:.0f}")
    grid = {
        "reject_radius_m": [34.0, 46.0, 60.0],
        "reach_m":         [110.0, 150.0],
        "omega":           [0.18, 0.30, 0.45],
        "densify_per_round": [0, 3, 6],
    }
    rows = []
    keys = list(grid)
    for combo in itertools.product(*[grid[k] for k in keys]):
        ov = dict(zip(keys, combo))
        try:
            fab = run(town, ov)
        except subprocess.CalledProcessError as e:
            print("FAILED", ov, e.stderr.decode()[:200]); continue
        # A config can produce a town with no blocks at all; its stats are
        # null and it simply scores badly.
        if any(fab.get(k) is None for k, _ in KEYS):
            print("  (degenerate)", ov)
            continue
        rows.append((score(fab, ref), ov, fab))
        f = fab
        print(f"  l0 {ov['reject_radius_m']:>4.0f}  reach {ov['reach_m']:>4.0f}  w {ov['omega']:.2f}  "
              f"| blocks {f['blocks']:>4}  med {f['block_area_median_m2']:>6.0f}  p90 {f['block_area_p90_m2']:>6.0f}  "
              f"M {f['meshedness']:>6.3f}  dead {f['dead_end_share']:.2f}  seg {f['segment_len_median_m']:>3.0f}  "
              f"dens {ov['densify_per_round']}  bldgs {f['buildings']:>4}  => {rows[-1][0]:.3f}")
    rows.sort(key=lambda r: r[0])
    print("\nbest five:")
    for s, ov, f in rows[:5]:
        print(f"  {s:.3f}  {ov}   blocks {f['blocks']} M {f['meshedness']:.3f} dead {f['dead_end_share']:.2f} bldgs {f['buildings']}")
