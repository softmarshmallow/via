#!/usr/bin/env python3
"""Independently re-measure a town from its raw Overpass extract, and compare
against via-bench's numbers (ADR 0009 Decision 4).

Why this exists: via-bench measures both sides of every comparison it
mediates through one code path (the same-code rule). What checks
via-bench itself is this script — the frozen protocol reimplemented from
the raw extract with the published toolchain (osmnx / networkx / shapely)
rather than with via-bench's code, so definition drift between our
implementation and the published ones is caught rather than shipped.
Independence is the whole point: this script never reads via-bench's
graph, only the same raw Overpass JSON, and the only via-bench output it
touches is the character table it is comparing against.

The protocol implemented here is research 0012 §3 (P1–P6), the same one
frozen in spikes/townfabric/VALIDATION.md:

- P1  statistics describe a disc of stated radius about a stated centre;
- P2  elements are loaded to buffer × radius and participate in the graph,
      statistics tally only interior elements (an edge counts when both
      its endpoint nodes are interior);
- P3  degree-2 vertices dissolved — here by osmnx's simplify_graph, which
      is the independent implementation this cross-check needs;
- P4  two street sets, reported separately (carriageway / all_ways);
- P5  local equirectangular projection about the study centre;
- P6  the output records extract hash and software versions.

No pass/fail judgment is made: per-character tolerances are declared in
the protocol freeze document (ADR 0010 Decision 3). This script only
reports positions and distances.

Usage (from the repo root):

    uv run --project analysis python analysis/cross_validate.py \
        --extract runs/reference/osm/alnwick-20260820T140000Z.json \
        --centre 55.4147,-1.7061 \
        --bench-json runs/reference/alnwick-fabric.json \
        --out runs/reference/alnwick-crossval.json

Relative paths are resolved against the repository root (the parent of
``analysis/``), never against the current working directory.
"""

import argparse
import json
import math
import sys
from pathlib import Path

import blake3
import networkx as nx
import osmnx as ox
import shapely
from shapely.geometry import Polygon

REPO_ROOT = Path(__file__).resolve().parent.parent

# P4 element sets. Carriageway: the drivable-plus-pedestrian street set,
# minus driveways and parking aisles. all_ways adds the footpath fabric.
CARRIAGEWAY_BASE = {
    "motorway", "trunk", "primary", "secondary", "tertiary",
    "unclassified", "residential", "living_street", "service", "pedestrian",
}
CARRIAGEWAY = CARRIAGEWAY_BASE | {f"{h}_link" for h in CARRIAGEWAY_BASE}
ALL_WAYS = CARRIAGEWAY | {"footway", "path", "steps", "cycleway"}
EXCLUDED_SERVICE = {"driveway", "parking_aisle"}

MIN_BUILDING_AREA_M2 = 8.0


def repo_path(p: str) -> Path:
    path = Path(p)
    return path if path.is_absolute() else REPO_ROOT / path


def blake3_file(path: Path) -> str:
    hasher = blake3.blake3()
    with open(path, "rb") as fh:
        while chunk := fh.read(1 << 20):
            hasher.update(chunk)
    return hasher.hexdigest()


def project(lat: float, lon: float, lat0: float, lon0: float) -> tuple[float, float]:
    """P5: local equirectangular about the study centre, in metres."""
    x = (lon - lon0) * 111320.0 * math.cos(math.radians(lat0))
    y = (lat - lat0) * 110540.0
    return x, y


def is_street(tags: dict, street_set: str) -> bool:
    highway = tags.get("highway")
    wanted = CARRIAGEWAY if street_set == "carriageway" else ALL_WAYS
    if highway not in wanted:
        return False
    if tags.get("service") in EXCLUDED_SERVICE:
        return False
    return True


def percentile(values: list[float], q: float) -> float | None:
    """Linear-interpolation percentile (numpy's default method)."""
    if not values:
        return None
    vals = sorted(values)
    h = (len(vals) - 1) * q
    lo = math.floor(h)
    hi = math.ceil(h)
    return vals[lo] + (vals[hi] - vals[lo]) * (h - lo)


def gini(values: list[float]) -> float | None:
    if not values:
        return None
    vals = sorted(values)
    n = len(vals)
    total = sum(vals)
    if total == 0:
        return 0.0
    cum = sum((i + 1) * v for i, v in enumerate(vals))
    return 2.0 * cum / (n * total) - (n + 1) / n


def build_street_graph(
    nodes_xy: dict[int, tuple[float, float]],
    ways: list[dict],
    street_set: str,
    r_buf: float,
) -> nx.MultiDiGraph:
    """Unsimplified street graph in osmnx format, clipped at r_buf (P2).

    A way is kept only if at least one vertex lies within r_buf of the
    centre; within a kept way, a segment is dropped when BOTH its
    endpoints are farther than r_buf.
    """
    # The crs attribute is metadata for osmnx's converters (which parse it
    # with pyproj); the coordinates themselves come from project(), the
    # protocol's own equirectangular formula.
    G = nx.MultiDiGraph(
        crs="+proj=eqc +lat_0=0 +lon_0=0 +units=m +ellps=WGS84 +no_defs",
        simplified=False,
    )
    for way in ways:
        tags = way.get("tags", {})
        if not is_street(tags, street_set):
            continue
        refs = [r for r in way.get("nodes", []) if r in nodes_xy]
        if len(refs) < 2:
            continue
        dists = {r: math.hypot(*nodes_xy[r]) for r in set(refs)}
        if min(dists[r] for r in refs) > r_buf:
            continue
        prev = None
        for ref in refs:
            if ref == prev:
                continue  # consecutive duplicate vertex
            if prev is not None and not (dists[prev] > r_buf and dists[ref] > r_buf):
                for node in (prev, ref):
                    if node not in G:
                        x, y = nodes_xy[node]
                        G.add_node(node, x=x, y=y)
                ax, ay = nodes_xy[prev]
                bx, by = nodes_xy[ref]
                length = math.hypot(bx - ax, by - ay)
                G.add_edge(prev, ref, length=length, osmid=way["id"])
                G.add_edge(ref, prev, length=length, osmid=way["id"])
            prev = ref
    return G


def bearing_entropy(bearings: list[float], num_bins: int = 36) -> float | None:
    """Shannon entropy (natural log) of bearings in 36 bins, the first bin
    spanning [-5°, 5°) — osmnx's documented binning, implemented directly
    because our graph is projected, not lat/lon."""
    if not bearings:
        return None
    counts = [0] * num_bins
    width = 360.0 / num_bins
    for b in bearings:
        counts[int(((b + width / 2) % 360.0) // width)] += 1
    total = sum(counts)
    return -sum((c / total) * math.log(c / total) for c in counts if c)


def measure_streets(G_und: nx.MultiGraph, radius: float) -> dict:
    """The character battery, tallied over interior elements (P2)."""
    pos = {n: (d["x"], d["y"]) for n, d in G_und.nodes(data=True)}
    interior = {n for n, (x, y) in pos.items() if math.hypot(x, y) <= radius}

    edges = []  # (u, v, length) over interior edges
    for u, v, data in G_und.edges(data=True):
        if u in interior and v in interior:
            edges.append((u, v, float(data["length"])))

    v_count = len(interior)
    e_count = len(edges)
    out: dict = {"nodes": v_count, "edges": e_count}
    if v_count == 0:
        return out

    out["street_km"] = sum(length for _, _, length in edges) / 1000.0
    out["k_avg"] = 2.0 * e_count / v_count
    out["edge_node_ratio"] = e_count / v_count
    denom = 2 * v_count - 5
    out["meshedness"] = (e_count - v_count + 1) / denom if denom > 0 else None

    # Degree on the full buffered graph (P2: the buffer exists exactly so
    # that streets leaving the disc are not counted as dead ends), tallied
    # over interior nodes.
    degree = dict(G_und.degree())
    out["dead_end_share"] = sum(1 for n in interior if degree[n] == 1) / v_count
    out["deg3_share"] = sum(1 for n in interior if degree[n] == 3) / v_count
    out["deg4_share"] = sum(1 for n in interior if degree[n] == 4) / v_count

    lengths = [length for _, _, length in edges]
    out["segment_len_median_m"] = percentile(lengths, 0.5)
    out["segment_len_p90_m"] = percentile(lengths, 0.9)

    # Circuity over interior edges; self-loops are excluded because their
    # straight-line endpoint distance is zero (osmnx does the same).
    len_sum = chord_sum = 0.0
    for u, v, length in edges:
        if u == v:
            continue
        (ux, uy), (vx, vy) = pos[u], pos[v]
        chord = math.hypot(vx - ux, vy - uy)
        if chord > 0:
            len_sum += length
            chord_sum += chord
    out["circuity_avg"] = len_sum / chord_sum if chord_sum > 0 else None

    out["self_loop_proportion"] = (
        sum(1 for u, v, _ in edges if u == v) / e_count if e_count else None
    )

    # Orientation: compass bearing atan2(dx, dy) of each interior edge's
    # endpoint chord, plus its reciprocal (Boeing 2019; osmnx represents
    # undirected edges bidirectionally). Unweighted, 36 bins.
    bearings: list[float] = []
    for u, v, _ in edges:
        if u == v:
            continue
        (ux, uy), (vx, vy) = pos[u], pos[v]
        dx, dy = vx - ux, vy - uy
        if dx == 0 and dy == 0:
            continue
        bearing = math.degrees(math.atan2(dx, dy)) % 360.0
        bearings.append(bearing)
        bearings.append((bearing + 180.0) % 360.0)
    entropy = bearing_entropy(bearings)
    out["orientation_entropy"] = entropy
    if entropy is None:
        out["orientation_order"] = None
    else:
        h_grid, h_max = math.log(4.0), math.log(36.0)
        out["orientation_order"] = 1.0 - ((entropy - h_grid) / (h_max - h_grid)) ** 2

    # Betweenness on the full buffered simplified graph (routes through the
    # boundary must exist — P2), reported over interior nodes.
    bc = nx.betweenness_centrality(G_und, weight="length", normalized=True)
    interior_bc = [bc[n] for n in interior]
    out["bc_gini"] = gini(interior_bc)
    out["bc_max"] = max(interior_bc)
    return out


def measure_buildings(
    nodes_xy: dict[int, tuple[float, float]],
    ways: list[dict],
    radius: float,
    r_buf: float,
) -> dict:
    """Closed building ways, centroid rule, area floor 8 m²."""
    interior_areas: list[float] = []
    for way in ways:
        tags = way.get("tags", {})
        if not tags.get("building") or tags.get("building") == "no":
            continue
        refs = way.get("nodes", [])
        if len(refs) < 4 or refs[0] != refs[-1]:
            continue  # not a closed way
        coords = [nodes_xy[r] for r in refs if r in nodes_xy]
        if len(coords) < 4:
            continue
        poly = Polygon(coords)
        if not poly.is_valid:
            poly = poly.buffer(0)
        if poly.is_empty or poly.area < MIN_BUILDING_AREA_M2:
            continue
        centroid = poly.centroid
        dist = math.hypot(centroid.x, centroid.y)
        if dist > r_buf:
            continue
        if dist <= radius:
            interior_areas.append(poly.area)
    return {
        "buildings": len(interior_areas),
        "footprint_area_median_m2": percentile(interior_areas, 0.5),
        "footprint_area_p90_m2": percentile(interior_areas, 0.9),
    }


def find_street_set_block(doc, street_set: str):
    """Depth-first search for a dict stored under the street-set key."""
    if isinstance(doc, dict):
        value = doc.get(street_set)
        if isinstance(value, dict):
            return value
        for child in doc.values():
            found = find_street_set_block(child, street_set)
            if found is not None:
                return found
    elif isinstance(doc, list):
        for child in doc:
            found = find_street_set_block(child, street_set)
            if found is not None:
                return found
    return None


def compare(ours: dict, bench_block: dict) -> dict:
    table = {}
    for key in sorted(ours):
        our_val = ours[key]
        bench_val = bench_block.get(key)
        row = {"ours": our_val, "bench": bench_val, "abs_diff": None, "rel_diff": None}
        if isinstance(our_val, (int, float)) and isinstance(bench_val, (int, float)):
            row["abs_diff"] = abs(our_val - bench_val)
            if bench_val != 0:
                row["rel_diff"] = row["abs_diff"] / abs(bench_val)
            elif our_val == 0:
                row["rel_diff"] = 0.0
        table[key] = row
    return table


def fmt(value) -> str:
    if value is None:
        return "-"
    if isinstance(value, int):
        return str(value)
    return f"{value:.6g}"


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Cross-validate via-bench against an independent "
        "osmnx/networkx re-measurement of the same extract."
    )
    ap.add_argument("--extract", required=True,
                    help="the Overpass JSON file via-bench measured")
    ap.add_argument("--centre", required=True, help='study centre as "lat,lon"')
    ap.add_argument("--radius", type=float, default=300.0,
                    help="study disc radius in metres (P1; default 300)")
    ap.add_argument("--buffer", type=float, default=1.45,
                    help="buffer factor (P2; default 1.45)")
    ap.add_argument("--bench-json", required=True,
                    help="via-bench's <town>-fabric.json output")
    ap.add_argument("--street-set", choices=["carriageway", "all_ways"],
                    default="carriageway", help="P4 element set (default carriageway)")
    ap.add_argument("--out", required=True, help="comparison JSON output path")
    args = ap.parse_args()

    lat0, lon0 = (float(part) for part in args.centre.split(","))
    r_buf = args.radius * args.buffer

    extract_path = repo_path(args.extract)
    doc = json.loads(extract_path.read_bytes())
    elements = doc.get("elements", [])
    nodes_xy = {
        el["id"]: project(el["lat"], el["lon"], lat0, lon0)
        for el in elements
        if el.get("type") == "node"
    }
    ways = [el for el in elements if el.get("type") == "way"]

    G = build_street_graph(nodes_xy, ways, args.street_set, r_buf)
    if G.number_of_nodes() == 0:
        print("error: no street fabric inside the buffered disc", file=sys.stderr)
        return 1
    unsimplified = (G.number_of_nodes(), G.number_of_edges())

    # P3, through osmnx's own implementation — the independence this
    # cross-check exists to provide. remove_rings=False: the protocol
    # dissolves degree-2 vertices, nothing more.
    G_simp = ox.simplify_graph(G, remove_rings=False)
    G_und = ox.convert.to_undirected(G_simp)

    ours = measure_streets(G_und, args.radius)
    ours.update(measure_buildings(nodes_xy, ways, args.radius, r_buf))

    bench_path = repo_path(args.bench_json)
    bench_doc = json.loads(bench_path.read_bytes())
    bench_block = find_street_set_block(bench_doc, args.street_set)
    if bench_block is None:
        print(
            f'error: no "{args.street_set}" block found in {bench_path}',
            file=sys.stderr,
        )
        return 1

    table = compare(ours, bench_block)

    result = {
        "protocol": {
            "centre": [lat0, lon0],
            "study_radius_m": args.radius,
            "buffer_factor": args.buffer,
            "street_set": args.street_set,
            "projection": "local equirectangular about the study centre (P5)",
            "simplification": "osmnx.simplify_graph, degree-2 dissolution (P3)",
            "interior_rule": "elements tallied within the study radius; an edge "
            "counts when both endpoint nodes are interior (P2)",
        },
        "provenance": {
            "extract": str(args.extract),
            "extract_blake3": blake3_file(extract_path),
            "bench_json": str(args.bench_json),
            "software": {
                "python": sys.version.split()[0],
                "osmnx": ox.__version__,
                "networkx": nx.__version__,
                "shapely": shapely.__version__,
            },
        },
        "diagnostics": {
            "buffered_unsimplified": {
                "nodes": unsimplified[0], "edges": unsimplified[1],
            },
            "buffered_simplified": {
                "nodes": G_und.number_of_nodes(),
                "edges": G_und.number_of_edges(),
            },
        },
        "comparison": table,
    }

    out_path = repo_path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")

    rows = sorted(
        table.items(),
        key=lambda kv: (kv[1]["rel_diff"] is None, -(kv[1]["rel_diff"] or 0.0)),
    )
    name_w = max(len(k) for k in table)
    header = f"{'character':<{name_w}}  {'ours':>12}  {'bench':>12}  {'abs_diff':>12}  {'rel_diff':>10}"
    print(header)
    print("-" * len(header))
    for key, row in rows:
        print(
            f"{key:<{name_w}}  {fmt(row['ours']):>12}  {fmt(row['bench']):>12}  "
            f"{fmt(row['abs_diff']):>12}  {fmt(row['rel_diff']):>10}"
        )
    print(
        "\nNo pass/fail is implied: per-character tolerances are declared in "
        "the protocol freeze document (ADR 0010 Decision 3)."
    )
    print(f"comparison written to {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
