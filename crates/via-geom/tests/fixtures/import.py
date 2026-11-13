"""Makes linework_a.json and course_a.json from fixtures/kernel/townland (spec/geometry.md 6).

The import rule, for this oracle data only: each coordinate in metres is converted to millimetres
and rounded to the nearest integer, ties to even, once. Source ids are renamed to neutral ids in
file order (`p000`, ...), so no fixture carries a library word. Run from the repository root:

    python3 crates/via-geom/tests/fixtures/import.py
"""

import json
from decimal import Decimal, ROUND_HALF_EVEN
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SRC = ROOT / "fixtures" / "kernel" / "townland"
OUT = Path(__file__).resolve().parent


def mm(v):
    return int((Decimal(v) * 1000).quantize(Decimal(1), rounding=ROUND_HALF_EVEN))


def lines(name):
    doc = json.loads((SRC / name).read_text(), parse_float=Decimal)
    out = []
    for f in doc["features"]:
        g = f["geometry"]
        parts = [g["coordinates"]] if g["type"] == "LineString" else g["coordinates"]
        for coords in parts:
            pts = []
            for x, y in coords:
                p = [mm(x), mm(y)]
                if not pts or pts[-1] != p:
                    pts.append(p)
            out.append(pts)
    return out


def write(name, paths, prefix):
    shapes = [{"source": f"{prefix}{i:03d}", "path": p} for i, p in enumerate(paths) if len(p) >= 2]
    (OUT / name).write_text(json.dumps({"shapes": shapes}, separators=(",", ":")) + "\n")
    return len(shapes)


print("linework_a", write("linework_a.json", lines("streets.geojson"), "p"))
print("course_a", write("course_a.json", lines("river.geojson"), "c"))
