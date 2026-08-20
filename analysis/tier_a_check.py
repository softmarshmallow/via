#!/usr/bin/env python3
"""Tier A cross-validation: formula-level check on via-bench's exported graph.

The protocol freeze (reference/protocol-v1.md, "Cross-validation
tolerances") declares two tiers of check against osmnx/networkx.
Tier B is cross_validate.py: an independent graph *construction* from
the raw Overpass extract, where systematic construction differences are
expected and bounded by per-character envelopes. Tier A is this script:
the **identical exported graph** — via-bench's own node and chain lists
from ``<town>-graph.json`` — re-measured with networkx, where every
character computable on the exported graph must match exactly
(default tolerance 1e-6). There is no construction step on this side,
so a Tier A disagreement is a formula defect in one of the two
implementations, never a construction difference.

Characters checked (everything the exported graph determines):
node/edge counts, k_avg, edge_node_ratio, meshedness, dead-end/deg3/deg4
shares, bc_gini, bc_max. Interior rule mirrors via-bench (P2): nodes are
interior per the exported flag; a chain counts when both its endpoints
are interior; degrees are taken on the whole exported (buffered) graph
and tallied over interior nodes.

Usage (from the repo root):

    uv run --project analysis python analysis/tier_a_check.py \
        --graph-json runs/reference/measured-v1/alnwick-graph.json \
        --bench-json runs/reference/measured-v1/alnwick-fabric.json \
        --street-set both

Exit status: 0 when every compared character agrees within --tolerance,
1 when any exceeds it (or cannot be compared), 2 on input errors.
Relative paths are resolved against the repository root (the parent of
``analysis/``), never against the current working directory.
"""

import argparse
import json
import sys
from pathlib import Path

import networkx as nx

REPO_ROOT = Path(__file__).resolve().parent.parent

# Deterministic reporting order for --street-set both.
STREET_SETS = ("carriageway", "all_ways")


def repo_path(p: str) -> Path:
    path = Path(p)
    return path if path.is_absolute() else REPO_ROOT / path


def gini(values: list[float]) -> float | None:
    """Gini coefficient: sort ascending, 1-based i,
    sum((2i - n - 1) * x_i) / (n * sum(x))."""
    if not values:
        return None
    vals = sorted(values)
    n = len(vals)
    total = sum(vals)
    if total == 0:
        return 0.0
    return sum((2 * i - n - 1) * v for i, v in enumerate(vals, start=1)) / (n * total)


def betweenness_graph(nodes: list[dict], chains: list[dict]) -> nx.Graph:
    """Simple undirected Graph over the exported chains, for betweenness.

    Every exported node is added (an isolated node has betweenness 0 and
    must still appear in the interior tally). Self-loop chains are
    skipped: a self-loop lies on no shortest path between distinct nodes,
    so it cannot carry betweenness. For parallel chains between the same
    node pair the SHORTER length is kept: on a shortest-path measure only
    the cheapest of a set of parallel edges can ever be traversed, so
    collapsing to the minimum preserves every shortest path while fitting
    networkx's simple-Graph input for betweenness_centrality.
    """
    G = nx.Graph()
    G.add_nodes_from(range(len(nodes)))
    for chain in chains:
        a, b, length = chain["a"], chain["b"], float(chain["len_m"])
        if a == b:
            continue
        if G.has_edge(a, b):
            if length < G[a][b]["length"]:
                G[a][b]["length"] = length
        else:
            G.add_edge(a, b, length=length)
    return G


def measure(block: dict) -> dict:
    """Recompute the graph-determined characters from an exported set."""
    nodes = block["nodes"]
    chains = block["chains"]
    interior = [i for i, nd in enumerate(nodes) if nd["interior"]]
    interior_set = set(interior)

    v_count = len(interior)
    e_count = sum(
        1 for ch in chains if ch["a"] in interior_set and ch["b"] in interior_set
    )
    out: dict = {"nodes": v_count, "edges": e_count}
    if v_count == 0:
        return out

    out["k_avg"] = 2.0 * e_count / v_count
    out["edge_node_ratio"] = e_count / v_count
    denom = 2 * v_count - 5
    out["meshedness"] = (e_count - v_count + 1) / denom if denom > 0 else None

    # Degrees on the whole exported (buffered) graph — a self-loop chain
    # contributes 2 to its node, the multigraph convention — tallied over
    # interior nodes (P2).
    degree = [0] * len(nodes)
    for chain in chains:
        degree[chain["a"]] += 1
        degree[chain["b"]] += 1
    out["dead_end_share"] = sum(1 for i in interior if degree[i] == 1) / v_count
    out["deg3_share"] = sum(1 for i in interior if degree[i] == 3) / v_count
    out["deg4_share"] = sum(1 for i in interior if degree[i] == 4) / v_count

    # Betweenness on the whole buffered graph (routes through the boundary
    # must exist — P2), reported over interior nodes.
    G = betweenness_graph(nodes, chains)
    bc = nx.betweenness_centrality(G, weight="length", normalized=True)
    interior_bc = [bc[i] for i in interior]
    out["bc_gini"] = gini(interior_bc)
    out["bc_max"] = max(interior_bc)
    return out


def find_bench_block(bench_doc: dict, street_set: str) -> dict | None:
    fabric = bench_doc.get("fabric")
    if isinstance(fabric, dict) and isinstance(fabric.get(street_set), dict):
        return fabric[street_set]
    if isinstance(bench_doc.get(street_set), dict):  # tolerate a bare layout
        return bench_doc[street_set]
    return None


def fmt(value) -> str:
    if value is None:
        return "-"
    if isinstance(value, int):
        return str(value)
    return f"{value:.9g}"


def compare_set(street_set: str, ours: dict, bench_block: dict, tolerance: float) -> int:
    """Print the comparison table for one street set; return failure count."""
    keys = sorted(ours)
    name_w = max(len(k) for k in keys + ["character"])
    header = (
        f"{'character':<{name_w}}  {'ours':>15}  {'bench':>15}  "
        f"{'abs_diff':>12}  status"
    )
    print(f"== {street_set} ==")
    print(header)
    print("-" * len(header))

    failures = 0
    for key in keys:
        our_val = ours[key]
        bench_val = bench_block.get(key)
        if isinstance(our_val, (int, float)) and isinstance(bench_val, (int, float)):
            diff = abs(our_val - bench_val)
            status = "ok" if diff <= tolerance else "FAIL"
        elif our_val is None and bench_val is None:
            diff, status = None, "ok"
        else:
            # One side has no number to compare — not verifiable, so a
            # failure at this tier (Tier A admits no missing characters).
            diff, status = None, "FAIL (not comparable)"
        if status != "ok":
            failures += 1
        print(
            f"{key:<{name_w}}  {fmt(our_val):>15}  {fmt(bench_val):>15}  "
            f"{fmt(diff):>12}  {status}"
        )
    print()
    return failures


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Tier A check: re-measure via-bench's exported graph "
        "with networkx and require exact agreement (protocol-v1, "
        "'Cross-validation tolerances')."
    )
    ap.add_argument("--graph-json", required=True,
                    help="via-bench's <town>-graph.json export "
                    "(per-street-set nodes and chains)")
    ap.add_argument("--bench-json", required=True,
                    help="via-bench's <town>-fabric.json character table")
    ap.add_argument("--street-set", choices=["carriageway", "all_ways", "both"],
                    default="both", help="which exported set(s) to check "
                    "(default both)")
    ap.add_argument("--tolerance", type=float, default=1e-6,
                    help="max allowed |ours - bench| per character "
                    "(default 1e-6, the Tier A bound)")
    args = ap.parse_args()

    graph_doc = json.loads(repo_path(args.graph_json).read_bytes())
    bench_doc = json.loads(repo_path(args.bench_json).read_bytes())

    wanted = STREET_SETS if args.street_set == "both" else (args.street_set,)

    total_failures = 0
    for street_set in wanted:
        graph_block = graph_doc.get(street_set)
        if not isinstance(graph_block, dict):
            print(
                f'error: no "{street_set}" graph in {args.graph_json}',
                file=sys.stderr,
            )
            return 2
        bench_block = find_bench_block(bench_doc, street_set)
        if bench_block is None:
            print(
                f'error: no "{street_set}" character block in {args.bench_json}',
                file=sys.stderr,
            )
            return 2
        total_failures += compare_set(
            street_set, measure(graph_block), bench_block, args.tolerance
        )

    if total_failures:
        print(
            f"TIER A FAIL: {total_failures} character(s) beyond "
            f"tolerance {args.tolerance:g} — a formula defect, not a "
            "construction difference (identical graph on both sides)."
        )
        return 1
    print(f"Tier A ok: all characters within tolerance {args.tolerance:g}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
