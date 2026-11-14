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


def lint(step):
    xs, ys = [8000, 4000], [8000, 4000]
    got = step.call("derive.descend", field="height", x=i32(xs), y=i32(ys))
    found = lines(got)
    rows = []
    for k in range(len(xs)):
        rows.append({"code": "descend_%d" % k,
                     "data": {"points": [c for p in found[k] for c in p],
                              "end": got["end"]["values"][k],
                              "geometry": int(got["geometry"]["values"][k])}})
    emit(step, rows)
