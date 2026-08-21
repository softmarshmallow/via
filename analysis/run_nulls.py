#!/usr/bin/env python3
"""Run class-matched null-model ensembles (ADR 0008 D9).

For each reference class, the three via-bench null models are run with
parameters matched to the class's **fitted-partition** population
(held-out towns fit nothing, D6), reading medians from the
populations.json that `via-bench corpus` wrote. Matching is density
matching, documented here and recorded in every output:

- **grid**: spacing s = r * sqrt(pi / n) puts ~n lattice intersections
  in the interior disc of radius r (the tally region), n = the class's
  median interior node count on its operative street set.
- **random**: nodes are scattered over the buffered extent (1.45 r),
  so the first guess is n * 1.45^2, corrected once by the achieved
  interior node count of a probe ensemble (one recorded rescale, no
  further iteration); target edge/node ratio = the class median.
- **dla**: particles start at n (a DLA tree adds ~one node per
  attached particle, growth is centre-weighted), corrected by the
  same single-rescale rule.

Every ensemble is written to ``<out>/<class-slug>/null-<model>.json``
by via-bench itself (its provenance block records model, params, seeds,
code revision); this driver additionally writes ``matching.json`` per
class — target, formula inputs, probe result, final params — so the
matching derivation is reproducible from its own record.

Usage (from the repo root, after ``via-bench corpus``):

    uv run --project analysis python analysis/run_nulls.py \
        [--populations runs/reference/report-v0/populations.json] \
        [--out runs/reference/nulls-v1.1] [--seeds 8]
"""

import argparse
import json
import math
import subprocess
import sys

from fetch_extract import REPO_ROOT, repo_path

BENCH = REPO_ROOT / "target" / "release" / "via-bench"
RADIUS = 300.0
BUFFER = 1.45


def run_null(out_dir, model: str, seeds: int, protocol_id: str, **params) -> dict:
    """One via-bench null ensemble; returns the parsed output JSON."""
    cmd = [
        str(BENCH), "null", "--model", model,
        "--out", str(out_dir),
        "--seeds", str(seeds),
        "--protocol-id", protocol_id,
    ]
    for k, v in params.items():
        cmd.append(f"--{k.replace('_', '-')}={v}")
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"null {model} failed: {proc.stderr.strip()[-400:]}")
    return json.loads((out_dir / f"null-{model}.json").read_text(encoding="utf-8"))


def achieved_nodes(doc: dict) -> float:
    return float(doc["ensemble"]["mean"]["nodes"])


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "--populations", default="runs/reference/report-v0/populations.json"
    )
    ap.add_argument("--out", default="runs/reference/nulls-v1.1")
    ap.add_argument("--seeds", type=int, default=8)
    ap.add_argument("--protocol-id", default="v1.1")
    args = ap.parse_args()

    if not BENCH.exists():
        print(f"error: {BENCH} not built", file=sys.stderr)
        return 1
    pops = json.loads(repo_path(args.populations).read_text(encoding="utf-8"))

    for slug, cls in pops["classes"].items():
        operative = cls["operative_set"]
        chars = cls["sets"][operative]
        n_med = chars["nodes"]["median"]
        enr_med = chars["edge_node_ratio"]["median"]
        if n_med is None or enr_med is None:
            print(f"{slug}: no population medians, skipped", file=sys.stderr)
            continue
        out_dir = repo_path(args.out) / slug
        out_dir.mkdir(parents=True, exist_ok=True)
        matching: dict = {
            "class": cls["class"],
            "operative_set": operative,
            "target_interior_nodes_median": n_med,
            "target_edge_node_ratio_median": enr_med,
            "protocol_id": args.protocol_id,
            "models": {},
        }

        spacing = RADIUS * math.sqrt(math.pi / n_med)
        doc = run_null(out_dir, "grid", args.seeds, args.protocol_id, spacing=spacing)
        matching["models"]["grid"] = {
            "spacing_m": spacing,
            "formula": "s = r*sqrt(pi/n)",
            "achieved_nodes_mean": achieved_nodes(doc),
        }
        print(f"{slug} grid: spacing {spacing:.1f} m -> {achieved_nodes(doc):.0f} nodes "
              f"(target {n_med:.0f})", flush=True)

        n0 = round(n_med * BUFFER * BUFFER)
        probe = run_null(
            out_dir, "random", args.seeds, args.protocol_id,
            nodes=n0, target_enr=enr_med,
        )
        a0 = achieved_nodes(probe)
        n1 = max(8, round(n0 * n_med / a0)) if a0 > 0 else n0
        doc = run_null(
            out_dir, "random", args.seeds, args.protocol_id,
            nodes=n1, target_enr=enr_med,
        )
        matching["models"]["random"] = {
            "nodes_probe": n0, "probe_achieved": a0, "nodes_final": n1,
            "target_enr": enr_med, "achieved_nodes_mean": achieved_nodes(doc),
        }
        print(f"{slug} random: {n0} -> {n1} scatter -> {achieved_nodes(doc):.0f} nodes "
              f"(target {n_med:.0f})", flush=True)

        p0 = max(20, round(n_med))
        probe = run_null(out_dir, "dla", args.seeds, args.protocol_id, particles=p0)
        a0 = achieved_nodes(probe)
        p1 = max(20, round(p0 * n_med / a0)) if a0 > 0 else p0
        doc = run_null(out_dir, "dla", args.seeds, args.protocol_id, particles=p1)
        matching["models"]["dla"] = {
            "particles_probe": p0, "probe_achieved": a0, "particles_final": p1,
            "achieved_nodes_mean": achieved_nodes(doc),
        }
        print(f"{slug} dla: {p0} -> {p1} particles -> {achieved_nodes(doc):.0f} nodes "
              f"(target {n_med:.0f})", flush=True)

        (out_dir / "matching.json").write_text(
            json.dumps(matching, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
