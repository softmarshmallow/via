"""Make the kernel fixtures `linework_a` and `course_a` from `fixtures/kernel/townland/` (geometry.md 6).

    python3 spec/vectors/geometry/import.py           # rewrite the fixtures
    python3 spec/vectors/geometry/import.py --check   # exit 1 if a fixture differs from what this makes

The import rule, for this oracle data only (geometry.md 6): each coordinate in metres, as written in
the GeoJSON text, is converted to millimetres and rounded to the nearest integer, ties to even, once.
The text is read as decimals, never as doubles, so the one rounding is the only one.

The grid is townland's own (spike-final:spike/worlds/townland/world.toml: extent 0 to 1600 m, resolution 2 m),
written in millimetres as `via kernel` reads it (geometry.md 7.1). Shapes past the extent are kept
whole (geometry.md 2.8).

Files written, each a `via-kernel-1` input (geometry.md 7.1), canonical JSON (documents.md 8):

| File | Holds |
|---|---|
| `linework_a.json` | Every line of `streets.geojson` as a `path`, its `id` the source, in the file's order. |
| `linework_a_shuffled.json` | The same shapes in another order: sorted by the bytes of the reversed source id. |
| `names_a.json` | `linework_a` as a `names` input (geometry.md 7.3): `params.of = "network"`, producer `streets`, no instances or edits (identity.md 47). |
| `names_a_moved.json` | `names_a` with one vertex of one source moved 1 m east (identity.md 47): the middle vertex of the source whose nearest vertex is farthest from the grid's centre. The script refuses a move that touches a junction, adds or loses a crossing, or comes within 2 m of another vertex. |
| `names_a_reversed.json` | `names_a` with the list of shapes in reverse order (identity.md 47). |
| `course_a.json` | The one line of `river.geojson` as a `path`, source `course_a`, with no grid or params: a case adds the stroke's. |

Python's standard library only. No core crate reads these files or names them (geometry.md 6).
"""

from __future__ import annotations

import json
import sys
from decimal import ROUND_HALF_EVEN, Decimal
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent.parent
TOWNLAND = REPO / "fixtures" / "kernel" / "townland"
GRID = {"extent": [0, 0, 1600000, 1600000], "resolution": 2000}


def mm(v: Decimal) -> int:
    return int((v * 1000).quantize(Decimal(1), rounding=ROUND_HALF_EVEN))


def lines(name: str) -> list[tuple[str, list[list[int]]]]:
    doc = json.loads((TOWNLAND / name).read_text(encoding="utf-8"), parse_float=Decimal, parse_int=Decimal)
    out = []
    for f in doc["features"]:
        g = f["geometry"]
        if g["type"] != "LineString":
            raise SystemExit(f"{name}: {f['properties'].get('id')} is a {g['type']}, not a LineString")
        out.append((str(f["properties"]["id"]), [[mm(x), mm(y)] for x, y in g["coordinates"]]))
    return out


def canonical(obj) -> bytes:
    return (json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode("utf-8")


def kernel_input(shapes: list[tuple[str, list[list[int]]]], grid: bool = True, params: dict | None = None) -> bytes:
    doc: dict = {"@format": "via-kernel-1", "shapes": [{"path": p, "source": s} for s, p in shapes]}
    if grid:
        doc["grid"] = GRID
    if params is not None:
        doc["params"] = params
    return canonical(doc)


NAMES = {"of": "network", "producer": "streets"}


def _orient(a, b, c) -> int:
    v = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    return (v > 0) - (v < 0)


def _on(a, b, p) -> bool:
    return min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1])


def _meet(a, b, c, d) -> bool:
    """Closed segments ab and cd share a point, exactly."""
    o1, o2, o3, o4 = _orient(a, b, c), _orient(a, b, d), _orient(c, d, a), _orient(c, d, b)
    if o1 != o2 and o3 != o4:
        return True
    return ((o1 == 0 and _on(a, b, c)) or (o2 == 0 and _on(a, b, d))
            or (o3 == 0 and _on(c, d, a)) or (o4 == 0 and _on(c, d, b)))


def _dist2_point_segment(p, a, b) -> float:
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)))
    qx, qy = a[0] + t * dx - p[0], a[1] + t * dy - p[1]
    return qx * qx + qy * qy


def _check_move(shapes, src: str, k: int, new: list[int]) -> None:
    """identity.md 47: not a junction, no crossing added or lost, no vertex passing another."""
    path = dict(shapes)[src]
    old = path[k]
    if not 0 < k < len(path) - 1:
        raise SystemExit("the moved vertex is a source's end")
    if sum(v == old for _, p in shapes for v in p) != 1:
        raise SystemExit("the moved vertex is shared")
    prev, nxt = path[k - 1], path[k + 1]
    mine = [(prev, old), (old, nxt), (prev, new), (new, nxt)]
    for s, p in shapes:
        for i in range(len(p) - 1):
            if s == src and i in (k - 2, k - 1, k, k + 1):
                continue
            c, d = p[i], p[i + 1]
            for a, b in mine:
                if _meet(a, b, c, d):
                    raise SystemExit(f"the move touches {s} segment {i}")
        for i, v in enumerate(p):
            if s == src and abs(i - k) <= 1:
                continue
            for a, b in mine:
                if _dist2_point_segment(v, a, b) < 2000.0 ** 2:
                    raise SystemExit(f"the move comes within 2 m of {s} vertex {i}")


def moved(shapes: list[tuple[str, list[list[int]]]]) -> list[tuple[str, list[list[int]]]]:
    """One vertex of one distant source moved 1 m east: the source whose nearest vertex is farthest
    from the grid's centre (ties by id bytes), its middle vertex (an interior one, not an end)."""
    cx = (GRID["extent"][0] + GRID["extent"][2]) // 2
    cy = (GRID["extent"][1] + GRID["extent"][3]) // 2

    def near(p: list[list[int]]) -> int:
        return min((x - cx) ** 2 + (y - cy) ** 2 for x, y in p)

    cands = [(s, p) for s, p in shapes if len(p) >= 3]
    src, path = max(cands, key=lambda sp: (near(sp[1]), [-b for b in sp[0].encode()]))
    k = len(path) // 2
    new = [path[k][0] + 1000, path[k][1]]
    _check_move(shapes, src, k, new)
    out = []
    for s, p in shapes:
        if s == src:
            p = [list(v) for v in p]
            p[k] = new
        out.append((s, p))
    return out


def files() -> dict[str, bytes]:
    streets = lines("streets.geojson")
    ids = [s for s, _ in streets]
    if len(set(ids)) != len(ids):
        raise SystemExit("streets.geojson: two lines share an id")
    river = lines("river.geojson")
    if len(river) != 1:
        raise SystemExit(f"river.geojson: {len(river)} lines, not one")
    return {
        "linework_a.json": kernel_input(streets),
        "linework_a_shuffled.json": kernel_input(sorted(streets, key=lambda sp: sp[0][::-1].encode())),
        "names_a.json": kernel_input(streets, params=NAMES),
        "names_a_moved.json": kernel_input(moved(streets), params=NAMES),
        "names_a_reversed.json": kernel_input(list(reversed(streets)), params=NAMES),
        "course_a.json": kernel_input([("course_a", river[0][1])], grid=False),
    }


def main(argv: list[str]) -> int:
    check = "--check" in argv
    bad = 0
    for name, data in files().items():
        path = HERE / name
        if check:
            if not path.is_file() or path.read_bytes() != data:
                print(f"{path.relative_to(REPO)}: differs from what import.py makes", file=sys.stderr)
                bad = 1
        else:
            path.write_bytes(data)
            print(f"wrote {path.relative_to(REPO)} ({len(data)} bytes)")
    return bad


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
