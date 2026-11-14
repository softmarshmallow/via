import json


def emit(step, rows):
    """diag.emit: one warning per row, `data` as canonical JSON text."""
    def col(values):
        return {"dtype": "str", "values": list(values)}

    step.call(
        "diag.emit",
        severity=col("warning" for _ in rows),
        code=col(r["code"] for r in rows),
        subject=col("world" for _ in rows),
        message=col(r["code"] for r in rows),
        hint=col("derived.md conformance" for _ in rows),
        data=col(json.dumps(r["data"], sort_keys=True, separators=(",", ":")) for r in rows),
        explain=col("A measure the derived chapter's conformance case reads." for _ in rows),
    )


def i32(values):
    return {"dtype": "i32", "values": list(values)}


def lines(result):
    """The vertices of each feature of a feature list (protocol.md 4.3), as [x, y] millimetres."""
    def ints(c):
        return [int(v) for v in c["values"]]

    flat = result["coords"]["values"]
    coords = flat if flat and isinstance(flat[0], list) else [flat[k:k + 2] for k in range(0, len(flat), 2)]
    pieces, rings, starts = ints(result["pieces"]), ints(result["rings"]), ints(result["ring_starts"])
    out = []
    for f in range(len(pieces) - 1):
        r = rings[pieces[f]]
        out.append([list(map(int, coords[k])) for k in range(starts[r], starts[r + 1])])
    return out


def toml_points(pts):
    return "[" + ", ".join("[%s, %s]" % (x / 1000, y / 1000) for x, y in pts) + "]"


def generate(step):
    w = step.call("derive.where", formulas={"dtype": "str", "values": ["= 1.5 - f('height')"]},
                  holds={"dtype": "i32", "shape": [1, 2], "values": [2000, 4000]}, view="head")
    shore = lines(w)[0]
    waypoints = {"coords": {"dtype": "i32", "shape": [2, 2], "values": [0, 0, 2000, 1000]},
                 "geometry": {"dtype": "u8", "values": [1]},
                 "pieces": {"dtype": "i64", "values": [0, 1]},
                 "rings": {"dtype": "i64", "values": [0, 1]},
                 "ring_starts": {"dtype": "i64", "values": [0, 2]},
                 "name": {"dtype": "str", "values": ["leg"]}}
    r = step.call("derive.route", cost="= 1", waypoints=waypoints, view="head")
    course = lines(r)[0]
    # The SDK's feature writer is M2 work; the document is TOML text either way (protocol.md 5).
    # Keys other than the text's `lake` and `course`, which the step sees (protocol.key_shadows).
    step.out._tables.append("[feature.pond]\nregion = %s\n" % toml_points(shore))
    step.out._tables.append("[feature.leg]\npath = %s\n" % toml_points(course))
