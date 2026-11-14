"""Kernel guarantees, checked from `via kernel` files alone (geometry.md 3.1, 3.2, 3.6, 7).

Each check recomputes what the chapter states from the output's own vertices and segments and the
input's shapes, in exact integer arithmetic where the chapter says "exactly", and compares. Nothing
here is a value recorded from a run. Also the `derive` action's writers (conformance.md 3.3): a
kernel input made from a kernel output, so a case can feed one call into the next.
"""

from __future__ import annotations

import math
from fractions import Fraction
from functools import cmp_to_key
from pathlib import Path
from typing import Any

from . import canon

Failure = tuple[str, str, str]
Pt = tuple[int, int]


class KernelFileError(Exception):
    """An input file of a kernel check is missing or not of the kernel's form."""


def read(path: Path) -> dict[str, Any]:
    try:
        doc = canon.loads(path.read_bytes())
    except (OSError, ValueError) as e:
        raise KernelFileError(f"cannot read {path.name}: {e}") from e
    if not isinstance(doc, dict):
        raise KernelFileError(f"{path.name} is not a JSON object")
    return doc


def _pt(v: Any) -> Pt:
    return (int(v[0]), int(v[1]))


# --- exact predicates ---------------------------------------------------------------------------


def orient(a: Pt, b: Pt, c: Pt) -> int:
    v = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    return (v > 0) - (v < 0)


def _in_box(a: Pt, b: Pt, p: Pt) -> bool:
    return min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1])


def on_segment(a: Pt, b: Pt, p: Pt) -> bool:
    return orient(a, b, p) == 0 and _in_box(a, b, p)


def meet(a: Pt, b: Pt, c: Pt, d: Pt) -> bool:
    """Closed segments ab and cd share at least one point."""
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    if o1 != o2 and o3 != o4 and 0 not in (o1, o2, o3, o4):
        return True
    return on_segment(a, b, c) or on_segment(a, b, d) or on_segment(c, d, a) or on_segment(c, d, b)


def twice_area(ring: list[Pt]) -> int:
    n = len(ring)
    return sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))


def dist2_point_segment(p: Pt, a: Pt, b: Pt) -> Fraction:
    dx, dy = b[0] - a[0], b[1] - a[1]
    ll = dx * dx + dy * dy
    if ll == 0:
        return Fraction((p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)
    t = Fraction((p[0] - a[0]) * dx + (p[1] - a[1]) * dy, ll)
    t = min(Fraction(1), max(Fraction(0), t))
    qx, qy = a[0] + t * dx - p[0], a[1] + t * dy - p[1]
    return qx * qx + qy * qy


def inside(p: Pt, ring: list[Pt]) -> bool:
    """Strictly inside a simple ring (even-odd); a point on it is not inside."""
    n = len(ring)
    hit = False
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if on_segment(a, b, p):
            return False
        if (a[1] > p[1]) != (b[1] > p[1]):
            # x of the crossing compared exactly: p.x < a.x + (p.y - a.y)(b.x - a.x)/(b.y - a.y)
            lhs = (p[0] - a[0]) * (b[1] - a[1])
            rhs = (p[1] - a[1]) * (b[0] - a[0])
            if (lhs < rhs) == (b[1] > a[1]):
                hit = not hit
    return hit


# --- noded --------------------------------------------------------------------------------------


def noded_faults(segs: list[tuple[Pt, Pt]], limit: int = 5) -> list[str]:
    """geometry.md 3.1 Noded: no two segments cross, overlap or touch except at a shared vertex."""
    faults: list[str] = []
    cell = 20000
    grid: dict[tuple[int, int], list[int]] = {}
    for k, (a, b) in enumerate(segs):
        if a == b:
            faults.append(f"segment {k} has zero length at {list(a)}")
            continue
        for i in range(min(a[0], b[0]) // cell, max(a[0], b[0]) // cell + 1):
            for j in range(min(a[1], b[1]) // cell, max(a[1], b[1]) // cell + 1):
                grid.setdefault((i, j), []).append(k)
    seen: set[tuple[int, int]] = set()
    for members in grid.values():
        for x in range(len(members)):
            for y in range(x + 1, len(members)):
                k, m = members[x], members[y]
                if (k, m) in seen:
                    continue
                seen.add((k, m))
                a, b = segs[k]
                c, d = segs[m]
                shared = {a, b} & {c, d}
                if len(shared) == 2:
                    faults.append(f"segments {k} and {m} are the same, {list(a)}-{list(b)}")
                elif len(shared) == 1:
                    s = shared.pop()
                    o = b if a == s else a
                    q = d if c == s else c
                    if on_segment(c, d, o) or on_segment(a, b, q):
                        faults.append(f"segments {k} and {m} overlap from {list(s)}")
                elif meet(a, b, c, d):
                    faults.append(f"segments {k} {list(a)}-{list(b)} and {m} {list(c)}-{list(d)} meet off a vertex")
                if len(faults) >= limit:
                    return faults
    return faults


# --- the fine graph, components and faces -------------------------------------------------------


def _half(v: Pt) -> int:
    return 0 if (v[1] > 0 or (v[1] == 0 and v[0] > 0)) else 1


def _angle_cmp(u: Pt, v: Pt) -> int:
    hu, hv = _half(u), _half(v)
    if hu != hv:
        return -1 if hu < hv else 1
    c = u[0] * v[1] - u[1] * v[0]
    return -1 if c > 0 else (1 if c < 0 else 0)


class Graph:
    def __init__(self, segs: list[tuple[Pt, Pt]]):
        self.segs = segs
        self.adj: dict[Pt, list[Pt]] = {}
        for a, b in segs:
            self.adj.setdefault(a, []).append(b)
            self.adj.setdefault(b, []).append(a)
        for v, ns in self.adj.items():
            ns.sort(key=cmp_to_key(lambda p, q, v=v: _angle_cmp((p[0] - v[0], p[1] - v[1]),
                                                                 (q[0] - v[0], q[1] - v[1]))))
        self.pos = {v: {n: i for i, n in enumerate(ns)} for v, ns in self.adj.items()}

    def components(self) -> dict[Pt, int]:
        comp: dict[Pt, int] = {}
        c = 0
        for v in sorted(self.adj):
            if v in comp:
                continue
            stack = [v]
            comp[v] = c
            while stack:
                u = stack.pop()
                for w in self.adj[u]:
                    if w not in comp:
                        comp[w] = c
                        stack.append(w)
            c += 1
        return comp

    def cycles(self) -> list[list[Pt]]:
        """Every face boundary, each with its face on the left: next(u->v) is v->w, w the neighbour
        of v just clockwise of u."""
        used: set[tuple[Pt, Pt]] = set()
        out: list[list[Pt]] = []
        for a, b in self.segs:
            for u, v in ((a, b), (b, a)):
                if (u, v) in used:
                    continue
                cyc: list[Pt] = []
                x, y = u, v
                while (x, y) not in used:
                    used.add((x, y))
                    cyc.append(x)
                    ns = self.adj[y]
                    w = ns[(self.pos[y][x] - 1) % len(ns)]
                    x, y = y, w
                out.append(cyc)
        return out


def canonical_ring(cyc: list[Pt]) -> list[Pt]:
    """geometry.md 2.5: no back-and-forth, no repeat, no vertex on the straight segment between its
    neighbours; from the least vertex."""
    r = list(cyc)
    changed = True
    while changed and len(r) >= 3:
        changed = False
        n = len(r)
        for i in range(n):
            a, b, c = r[i - 1], r[i], r[(i + 1) % n]
            if a == b or a == c or (orient(a, b, c) == 0 and _in_box(a, c, b)):
                del r[i]
                changed = True
                break
    k = r.index(min(r))
    return r[k:] + r[:k]


# --- network ------------------------------------------------------------------------------------


def _segments(out: dict[str, Any]) -> list[tuple[Pt, Pt]]:
    return [(_pt(s["ends"][0]), _pt(s["ends"][1])) for s in out.get("segments", [])]


def _sources(s: dict[str, Any]) -> frozenset[str]:
    return frozenset(p["source"] for p in s.get("provenance", []))


def path_ends(inp: dict[str, Any]) -> set[Pt]:
    ends: set[Pt] = set()
    for sh in inp.get("shapes", []):
        if "path" in sh and sh["path"]:
            ends.add(_pt(sh["path"][0]))
            ends.add(_pt(sh["path"][-1]))
    return ends


def network_valid(cid: str, out: dict[str, Any], inp: dict[str, Any]) -> list[Failure]:
    """geometry.md 3.2: Noded; Euler at both levels; the junctions and edges of its definition;
    every area valid and one of the faces; Closure, exactly."""
    fails: list[Failure] = []
    name = f"check.{cid}"
    segs = _segments(out)
    verts = [_pt(v) for v in out.get("vertices", [])]
    for f in noded_faults(segs):
        fails.append((name, "no two segments cross or touch off a vertex (3.1 Noded)", f))
    if sorted(set(verts)) != verts:
        fails.append((name, "vertices distinct, by (x, y) (2.5)", "not so"))
    g = Graph(segs)
    if set(g.adj) != set(verts):
        fails.append((name, "the vertices are exactly the segments' ends", f"{len(set(verts) ^ set(g.adj))} differ"))
    comp = g.components()
    C = len(set(comp.values()))
    if out.get("components") != C:
        fails.append((name, f"components {C}, counted from the segments", canon.show(out.get("components"))))
    areas = out.get("areas", [])
    A = len(areas)
    V, E = len(verts), len(segs)
    if V - E + A != C:
        fails.append((name, "Euler, fine: V - E + A = C", f"{V} - {E} + {A} != {C}"))

    # Junctions by their definition, edges as the chains between them.
    ends = path_ends(inp)
    by_pair = {frozenset(s): _sources(o) for s, o in zip(segs, out.get("segments", []))}
    junctions: set[Pt] = set()
    for v, ns in g.adj.items():
        if len(ns) != 2 or v in ends or by_pair[frozenset((v, ns[0]))] != by_pair[frozenset((v, ns[1]))]:
            junctions.add(v)
    seen: set[frozenset] = set()
    chains = 0
    for j in sorted(junctions):
        for n in g.adj[j]:
            if frozenset((j, n)) in seen:
                continue
            chains += 1
            x, y = j, n
            seen.add(frozenset((x, y)))
            while y not in junctions:
                a, b = g.adj[y]
                w = b if a == x else a
                x, y = y, w
                seen.add(frozenset((x, y)))
    for s in segs:  # rings of segments with no junction: one junction at the least vertex, one edge
        if frozenset(s) in seen:
            continue
        ring = [s[0]]
        x, y = s[0], s[1]
        seen.add(frozenset(s))
        while y != s[0]:
            ring.append(y)
            a, b = g.adj[y]
            w = b if a == x else a
            x, y = y, w
            seen.add(frozenset((x, y)))
        junctions.add(min(ring))
        chains += 1
    got_j = [_pt(p) for p in out.get("junctions", [])]
    if got_j != sorted(junctions):
        fails.append((name, f"{len(junctions)} junctions by 3.2's definition, by (x, y)",
                      f"{len(got_j)}; {len(set(got_j) ^ junctions)} differ"))
    J, Ed = len(got_j), len(out.get("edges", []))
    if Ed != chains:
        fails.append((name, f"{chains} edges, the chains between junctions", str(Ed)))
    if J - Ed + A != C:
        fails.append((name, "Euler, coarse: J - E + A = C", f"{J} - {Ed} + {A} != {C}"))

    # Faces: positive cycles are bounded faces' shells; each component has one outer cycle.
    cycles = g.cycles()
    shells = {tuple(canonical_ring(c)) for c in cycles if twice_area(c) > 0}
    if len(shells) != A:
        fails.append((name, f"{len(shells)} areas, the bounded faces", str(A)))
    vertex_set = set(verts)
    total = 0
    for k, a in enumerate(areas):
        region = a.get("region")
        if not isinstance(region, list) or len(region) != 1 or not region[0]:
            fails.append((name, f"area {k}: a region of one piece (3.2)", canon.show(region)[:80]))
            continue
        rings = [[_pt(p) for p in r] for r in region[0]]
        ta = 0
        for i, r in enumerate(rings):
            t = twice_area(r)
            if (t > 0) != (i == 0) or len(set(r)) != len(r) or r[0] != min(r) or not set(r) <= vertex_set:
                fails.append((name, f"area {k} ring {i}: canonical and valid (2.5): "
                                    f"{'shell counter-clockwise' if i == 0 else 'hole clockwise'}, no repeat, "
                                    "from its least vertex, on network vertices", f"twice-area {t}"))
            ta += t
        if tuple(rings[0]) not in shells:
            fails.append((name, f"area {k}: its shell is a face of the network", f"start {list(rings[0][0])}"))
        if a.get("twice_area") != ta:
            fails.append((name, f"area {k}: twice_area {ta}, from its rings", canon.show(a.get("twice_area"))))
        total += ta
        if len(fails) > 20:
            return fails

    # Closure: the areas' sum against the filled outline of each top-level component.
    outer: dict[int, int] = {}
    bounded: dict[int, list[list[Pt]]] = {}
    for c in cycles:
        t = twice_area(c)
        ci = comp[c[0]]
        if t > 0:
            bounded.setdefault(ci, []).append(c)
        else:
            outer[ci] = outer.get(ci, 0) + 1
            outer.setdefault(-1 - ci, 0)
            outer[-1 - ci] -= t
    if any(outer.get(ci) != 1 for ci in set(comp.values())):
        fails.append((name, "one outer boundary per component", "not so"))
    first = {}
    for v, ci in comp.items():
        first.setdefault(ci, v)
    enclosed = 0
    for ci, v in first.items():
        top = not any(inside(v, canonical_ring(c)) for cj, cs in bounded.items() if cj != ci for c in cs)
        if top:
            enclosed += outer.get(-1 - ci, 0)
    if total != enclosed:
        fails.append((name, f"Closure: the areas' twice-areas sum to {enclosed}, what the top-level "
                            "components enclose", str(total)))
    return fails


def network_measures(out: dict[str, Any], reference: dict[str, Any]) -> str:
    areas = out.get("areas", [])
    total = sum(Fraction(a.get("twice_area", 0), 2) for a in areas) / 1_000_000
    ref = reference.get("streets", {})
    return (f"measure (geometry.md 6, read, never tuned to): {len(areas)} areas against "
            f"{ref.get('faces')} faces; total area {float(total):.2f} m2 against "
            f"{ref.get('total_area_m2')} m2")


# --- noding -------------------------------------------------------------------------------------


def _source_segments(inp: dict[str, Any]) -> dict[str, list[tuple[Pt, Pt]]]:
    out: dict[str, list[tuple[Pt, Pt]]] = {}
    for sh in inp.get("shapes", []):
        if "path" in sh:
            p = [_pt(v) for v in sh["path"]]
            p = [v for i, v in enumerate(p) if i == 0 or v != p[i - 1]]
            out[sh["source"]] = list(zip(p, p[1:]))
        elif "ring" in sh:
            p = [_pt(v) for v in sh["ring"]]
            out[sh["source"]] = list(zip(p, p[1:] + p[:1]))
    return out


def noding_valid(cid: str, out: dict[str, Any], inp: dict[str, Any], bound: Fraction) -> list[Failure]:
    """geometry.md 3.1: Noded; Close (each segment within the bound of every source it names);
    Faithful (every source vertex within the bound of a segment naming it; each path's t covered
    from 0 to 1 without a gap)."""
    fails: list[Failure] = []
    name = f"check.{cid}"
    segs = _segments(out)
    for f in noded_faults(segs):
        fails.append((name, "no two segments cross or touch off a vertex (3.1 Noded)", f))
    src = _source_segments(inp)
    b2 = bound * bound
    naming: dict[str, list[tuple[Pt, Pt]]] = {}
    spans: dict[str, list[tuple[float, float]]] = {}
    for k, (s, o) in enumerate(zip(segs, out.get("segments", []))):
        for p in o.get("provenance", []):
            sid, i = p["source"], p["segment"]
            if sid not in src or not 0 <= i < len(src[sid]):
                fails.append((name, f"segment {k}: provenance names an input segment", canon.show(p)))
                continue
            a, b = src[sid][i]
            for e in s:
                if dist2_point_segment(e, a, b) > b2:
                    fails.append((name, f"segment {k} within {bound} mm of {sid} segment {i} (3.1 Close)",
                                  f"end {list(e)} at {math.sqrt(dist2_point_segment(e, a, b)):.3f} mm"))
            naming.setdefault(sid, []).append(s)
            t = p["t"]
            spans.setdefault(sid, []).append((min(t), max(t)))
        if len(fails) > 20:
            return fails
    for sid, ss in src.items():
        mine = naming.get(sid, [])
        for v in {v for seg in ss for v in seg}:
            if not any(dist2_point_segment(v, a, b) <= b2 for a, b in mine):
                fails.append((name, f"{sid}'s vertex {list(v)} within {bound} mm of a segment naming it "
                                    "(3.1 Faithful)", "none"))
                break
        if sid in {sh["source"] for sh in inp.get("shapes", []) if "path" in sh}:
            iv = sorted(spans.get(sid, []))
            at = 0.0
            for lo, hi in iv:
                if lo > at:
                    break
                at = max(at, hi)
            if at != 1.0:
                fails.append((name, f"{sid}'s t covered from 0 to 1 by its segments (3.1 Faithful)",
                              f"covered to {at!r}"))
    return fails


def noding_same(cid: str, a: dict[str, Any], b: dict[str, Any]) -> list[Failure]:
    """Idempotent (3.1): the same vertices and the same segments, byte for byte."""
    fails: list[Failure] = []
    if canon.encode(a.get("vertices")) != canon.encode(b.get("vertices")):
        fails.append((f"check.{cid}", "the same vertices after noding again (3.1 Idempotent)",
                      f"{len(a.get('vertices', []))} then {len(b.get('vertices', []))}"))
    ea = [s["ends"] for s in a.get("segments", [])]
    eb = [s["ends"] for s in b.get("segments", [])]
    if canon.encode(ea) != canon.encode(eb):
        fails.append((f"check.{cid}", "the same segments after noding again (3.1 Idempotent)",
                      f"{len(ea)} then {len(eb)}"))
    return fails


# --- names --------------------------------------------------------------------------------------


def names_same(cid: str, a: dict[str, Any], b: dict[str, Any]) -> list[Failure]:
    """identity.md 47: every junction, edge and area name unchanged."""
    fails: list[Failure] = []
    for k in ("junctions", "edges", "areas"):
        na = sorted(str(x.get("name")) for x in a.get(k, []))
        nb = sorted(str(x.get("name")) for x in b.get(k, []))
        if not na:
            fails.append((f"check.{cid}", f"{k} named in the first output", "none"))
        if na != nb:
            gone = sorted(set(na) - set(nb))[:3]
            new = sorted(set(nb) - set(na))[:3]
            fails.append((f"check.{cid}", f"the same {k} names", f"lost {gone}, gained {new}"))
    return fails


# --- cells --------------------------------------------------------------------------------------


def _along(p: Pt, q: Pt, u: Pt, v: Pt, tol: float) -> float:
    """Length of cell edge pq that runs along segment uv, both ends within `tol` mm of its line."""
    dx, dy = v[0] - u[0], v[1] - u[1]
    L = math.hypot(dx, dy)
    if L == 0:
        return 0.0
    for e in (p, q):
        if abs((e[0] - u[0]) * dy - (e[1] - u[1]) * dx) / L > tol:
            return 0.0
    sp = ((p[0] - u[0]) * dx + (p[1] - u[1]) * dy) / L
    sq = ((q[0] - u[0]) * dx + (q[1] - u[1]) * dy) / L
    lo, hi = max(min(sp, sq), 0.0), min(max(sp, sq), L)
    return max(0.0, hi - lo)


def _edge_paths(net: dict[str, Any]) -> dict[tuple[Pt, Pt], list[list[Pt]]]:
    out: dict[tuple[Pt, Pt], list[list[Pt]]] = {}
    for e in net.get("edges", []):
        out.setdefault((_pt(e["ends"][0]), _pt(e["ends"][1])), []).append([_pt(p) for p in e["path"]])
    return out


def cells_valid(cid: str, out: dict[str, Any], inp: dict[str, Any], net: dict[str, Any],
                bound: float) -> list[Failure]:
    """geometry.md 3.6: Fronts (every cell borders its frontage edge along at least
    min_cell_length, else geom.cell_small at it); Closure per area (cells plus rest equal the
    area, within what noding may move the outline: `bound` mm along its perimeter); no cell at
    all gives geom.cell_none."""
    fails: list[Failure] = []
    name = f"check.{cid}"
    params = inp.get("params", {})
    min_len = float(params.get("min_cell_length", 0))
    start = _pt(params["area"])
    area = next((a for a in net.get("areas", []) if _pt(a["region"][0][0][0]) == start), None)
    if area is None:
        raise KernelFileError(f"no area of the network starts at {list(start)}")
    diags = out.get("diagnostics", [])
    small = [d.get("position") for d in diags if d.get("code") == "geom.cell_small"]
    cells = out.get("cells", [])
    if not cells and not any(d.get("code") == "geom.cell_none" for d in diags):
        fails.append((name, "no cell, so geom.cell_none (3.6 step 4)", "no such record"))
    paths = _edge_paths(net)
    keys = [canon.encode([c.get("frontage"), c.get("station")]) for c in cells]
    if len(set(keys)) != len(keys):
        dup = next(k for k in keys if keys.count(k) > 1)
        fails.append((name, "one cell per (frontage edge, side, station): identity.md names a cell by them",
                      f"{keys.count(dup)} cells at {dup}"))
    total = 0
    for k, c in enumerate(cells):
        fr = c.get("frontage", {})
        key = (_pt(fr["edge"][0]), _pt(fr["edge"][1]))
        segs = [s for p in paths.get(key, []) for s in zip(p, p[1:])]
        region = [[[_pt(v) for v in r] for r in piece] for piece in c["region"]]
        ring_edges = [(r[i], r[(i + 1) % len(r)]) for piece in region for r in piece for i in range(len(r))]
        length = sum(_along(p, q, u, v, bound) for p, q in ring_edges for u, v in segs)
        if length < min_len - 1e-6:
            xs = [v[0] for piece in region for r in piece for v in r]
            ys = [v[1] for piece in region for r in piece for v in r]
            box = (min(xs) - 1000, min(ys) - 1000, max(xs) + 1000, max(ys) + 1000)
            reported = any(
                isinstance(pos, list) and len(pos) >= 2
                and box[0] <= pos[0] * 1000 <= box[2] and box[1] <= pos[1] * 1000 <= box[3]
                for pos in small
            )
            if not reported:
                fails.append((name, f"cell {k} borders {fr.get('source')} along at least {min_len:.0f} mm, "
                                    "or geom.cell_small at it (3.6 Fronts)", f"{length:.1f} mm"))
        total += sum(twice_area(r) for piece in region for r in piece)
    rest = out.get("rest", [])
    total += sum(twice_area([_pt(v) for v in r]) for piece in rest for r in piece)
    want = area["twice_area"]
    rings = [[_pt(v) for v in r] for r in area["region"][0]]
    perim = sum(math.dist(r[i], r[(i + 1) % len(r)]) for r in rings for i in range(len(r)))
    if abs(total - want) > 2 * bound * perim:
        fails.append((name, f"cells and rest sum to the area's twice-area {want}, within "
                            f"{2 * bound * perim:.0f} (3.6 Closure)", str(total)))
    return fails


# --- derive (conformance.md 3.3) ----------------------------------------------------------------


def derive_segments(out: dict[str, Any]) -> dict[str, Any]:
    """Noding's output as a noding input: each segment a two-point path, source `s<index>`."""
    n = len(out.get("segments", []))
    w = len(str(max(n - 1, 0)))
    return {
        "@format": "via-kernel-1",
        "shapes": [{"path": s["ends"], "source": f"s{k:0{w}d}"} for k, s in enumerate(out.get("segments", []))],
    }


def derive_cells(net: dict[str, Any], inp: dict[str, Any], params: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    """One `cells` input per area of a network (geometry.md Conformance, `linework_a`'s cells):
    the network's shapes and grid, the area by its start vertex, and as frontage every
    (source, edge, side) on its shell, the side facing the area."""
    segs = {frozenset((_pt(s["ends"][0]), _pt(s["ends"][1]))): s for s in net.get("segments", [])}
    out: list[tuple[str, dict[str, Any]]] = []
    areas = net.get("areas", [])
    w = len(str(max(len(areas) - 1, 0)))
    for k, a in enumerate(areas):
        shell = [_pt(v) for v in a["region"][0][0]]
        ring_edges = [(shell[i], shell[(i + 1) % len(shell)]) for i in range(len(shell))]
        front: set[tuple[str, Pt, Pt, str]] = set()
        for e in net.get("edges", []):
            path = [_pt(p) for p in e["path"]]
            u, v = path[0], path[1]
            on = [(p, q) for p, q in ring_edges if on_segment(p, q, u) and on_segment(p, q, v)]
            if not on:
                continue
            p, q = on[0]
            forward = (v[0] - u[0]) * (q[0] - p[0]) + (v[1] - u[1]) * (q[1] - p[1]) > 0
            s = segs[frozenset((u, v))]
            e0, e1 = _pt(s["ends"][0]), _pt(s["ends"][1])
            for pr in s.get("provenance", []):
                travel_uv = ((e0, e1) == (u, v)) != bool(pr.get("reversed"))
                side = "left" if travel_uv == forward else "right"
                front.add((pr["source"], _pt(e["ends"][0]), _pt(e["ends"][1]), side))
        doc = {
            "@format": "via-kernel-1",
            "grid": inp["grid"],
            "shapes": inp["shapes"],
            "params": dict(params, area=list(shell[0]), frontage=[
                {"edge": [list(a0), list(a1)], "side": sd, "source": src}
                for src, a0, a1, sd in sorted(front, key=lambda f: (f[0].encode(), f[1], f[2], f[3]))
            ]),
        }
        out.append((f"a{k:0{w}d}.json", doc))
    return out


def derive_interpolate(out: dict[str, Any], inp: dict[str, Any]) -> dict[str, Any]:
    """A `project` output as an `interpolate` input: the input's shapes, `t` each result's `t`."""
    ts = [r.get("t") for r in out.get("results", [])]
    if any(not isinstance(t, (int, float)) or isinstance(t, bool) for t in ts):
        raise KernelFileError("a project result has no t")
    return {"@format": "via-kernel-1", "shapes": inp["shapes"], "t": ts}


def derive_project(out: dict[str, Any], inp: dict[str, Any]) -> dict[str, Any]:
    """An `interpolate` output as a `project` input: the input's shapes, `points` each result's point."""
    return {"@format": "via-kernel-1", "shapes": inp["shapes"],
            "points": [list(_pt(r["point"])) for r in out.get("results", [])]}


def derive_params(inp: dict[str, Any], params: dict[str, Any]) -> dict[str, Any]:
    """A kernel input with its `params` replaced: the same shapes asked another way."""
    return dict(inp, params=params)


# --- linear referencing round trips (geometry.md 3.8) -------------------------------------------


def _line(inp: dict[str, Any]) -> list[tuple[Pt, Pt]]:
    """The one shape's segments, in travel order: a path's, or a ring's with the closing one."""
    shapes = inp.get("shapes", [])
    if len(shapes) != 1:
        raise KernelFileError(f"the input holds {len(shapes)} shapes, not one")
    s = shapes[0]
    if "path" in s:
        pts = [_pt(p) for p in s["path"]]
        return list(zip(pts, pts[1:]))
    if "ring" in s:
        pts = [_pt(p) for p in s["ring"]]
        return list(zip(pts, pts[1:] + pts[:1]))
    raise KernelFileError("the shape is not a path or a ring")


def exact_nearest(p: Pt, line: list[tuple[Pt, Pt]]) -> tuple[Fraction, Fraction]:
    """The nearest point of the line to `p`, exactly; on a tie the one first along the line."""
    best: tuple[Fraction, Fraction, Fraction] | None = None
    for a, b in line:
        dx, dy = b[0] - a[0], b[1] - a[1]
        ll = dx * dx + dy * dy
        if ll == 0:
            q = (Fraction(a[0]), Fraction(a[1]))
        else:
            u = Fraction((p[0] - a[0]) * dx + (p[1] - a[1]) * dy, ll)
            u = min(max(u, Fraction(0)), Fraction(1))
            q = (a[0] + u * dx, a[1] + u * dy)
        d2 = (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2
        if best is None or d2 < best[0]:
            best = (d2, q[0], q[1])
    assert best is not None
    return best[1], best[2]


def roundtrip_point(cid: str, out: dict[str, Any], inp: dict[str, Any], bound: Fraction) -> list[Failure]:
    """`interpolate(project(p))` lies within `bound` mm of the exact nearest point (3.8)."""
    name = f"check.{cid}"
    line = _line(inp)
    pts = [_pt(p) for p in inp.get("points", [])]
    res = out.get("results", [])
    if len(res) != len(pts) or not pts:
        return [(name, f"one interpolate result per point ({len(pts)})", str(len(res)))]
    worst, at = Fraction(0), -1
    for k, (p, r) in enumerate(zip(pts, res)):
        q = _pt(r["point"])
        nx, ny = exact_nearest(p, line)
        d2 = (q[0] - nx) ** 2 + (q[1] - ny) ** 2
        if d2 > worst:
            worst, at = d2, k
    if worst > bound * bound:
        p = pts[at]
        return [(name, f"every point within {float(bound)} mm of the exact nearest point",
                 f"point {at} {list(p)}: {res[at]['point']} lies {math.sqrt(worst):.4f} mm from it")]
    return []


def roundtrip_t(cid: str, out: dict[str, Any], inp: dict[str, Any], bound: float) -> list[Failure]:
    """`project(interpolate(t))` gives `t'` with `|t' - t| x L` at most `bound` mm (3.8)."""
    name = f"check.{cid}"
    line = _line(inp)
    length = sum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in line)
    ts = inp.get("t", [])
    res = out.get("results", [])
    if len(res) != len(ts) or not ts:
        return [(name, f"one project result per t ({len(ts)})", str(len(res)))]
    worst, at = 0.0, -1
    for k, (t, r) in enumerate(zip(ts, res)):
        if not isinstance(r.get("t"), (int, float)):
            return [(name, "every project result has a t", f"result {k}: {r.get('t')!r}")]
        e = abs(r["t"] - t) * length
        if e > worst:
            worst, at = e, k
    if worst > bound:
        return [(name, f"|t' - t| x L at most {bound} mm",
                 f"t {ts[at]!r} came back as {res[at]['t']!r}: {worst:.4f} mm")]
    return []


def region_twice_area(region: Any) -> int:
    """A region's twice-area: each piece's shell less its holes (holes run clockwise, 2.5)."""
    return sum(twice_area([_pt(v) for v in ring]) for piece in region for ring in piece)


def stroke_measures(out: dict[str, Any], reference: dict[str, Any], distance_m: float) -> str:
    offs = [o for o in reference.get("river_offsets", {}).get("offsets", [])
            if o.get("distance_m") == distance_m]
    ref = offs[0].get("area_m2") if offs else None
    areas = [float(Fraction(region_twice_area(r), 2) / 1_000_000) for r in out.get("results", [])]
    shown = ", ".join(f"{a:.2f}" for a in areas) or "none"
    return (f"measure (geometry.md 6, read, never tuned to): stroked area at {distance_m} m "
            f"{shown} m2 against {ref} m2")
