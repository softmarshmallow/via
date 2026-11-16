"""Run an export step over a finished build the way the engine will (protocol.md 2.2, plain
command), until the engine runs `when = "export"` itself (doc.not_yet in M6 slice 1).

    python tools/map_export.py BUILD --step STEP.toml --out FOLDER [--python PY]

STEP.toml holds one `[step.<id>]` table (`when = "export"`, `command`, `params`). The request's
`data` is built from BUILD alone: every field over the whole grid (a weights field's chunks joined,
one plane per material of the field, in id order), every feature with every attribute (protocol.md
4.3 columns; a lattice length is not told apart from a number in features.json, so every number is
f64 metres here), the instances and the pack's bounds and taxons. The step writes into FOLDER, which
stands for `export/<step id>/`. A stand-in: the engine's own runner replaces it (TODO "Player map").
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
import tomllib

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def col(values, dtype, shape=None):
    c = {"dtype": dtype, "values": values}
    if shape is not None:
        c["shape"] = shape
    return c


def blob(objects, arr):
    """A field as a file under objects, named by a digest stand-in (its bytes' SHA-256)."""
    import hashlib

    raw = arr.tobytes()
    d = hashlib.sha256(raw).hexdigest()
    path = os.path.join(objects, d[:2], d)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    np.save(path + ".npy", arr)
    os.replace(path + ".npy", path)
    return d, path


def features_columns(build):
    fj = json.load(open(os.path.join(build, "features.json")))
    rows = list(fj["features"])
    for fp in fj["footprints"]:
        rows.append(dict(fp, name=fp["instance"] + ".ground", shape="region"))
    rows.sort(key=lambda f: f["name"].encode())
    coords, geom, pieces, rings, starts = [], [], [0], [0], [0]

    def mm(v):
        return int(round(v * 1000))

    for f in rows:
        if f["shape"] == "region":
            geom.append(2)
            for pc in f["pieces"]:
                for ring in [pc["outer"]] + pc["holes"]:
                    coords.extend([mm(x), mm(y)] for x, y, *_ in ring)
                    starts.append(len(coords))
                rings.append(len(starts) - 1)
        else:
            geom.append(0 if f["shape"] == "point" else 1)
            pts = [f["at"]] if f["shape"] == "point" else f["points"]
            coords.extend([mm(p[0]), mm(p[1])] for p in pts)
            starts.append(len(coords))
            rings.append(len(starts) - 1)
        pieces.append(len(rings) - 1)
    cols = {"coords": col(coords, "i32", [len(coords), 2]), "geometry": col(geom, "u8"),
            "pieces": col(pieces, "i64"), "rings": col(rings, "i64"),
            "ring_starts": col(starts, "i64"),
            "name": col([f["name"] for f in rows], "str"),
            "kind": col([f.get("kind") or "" for f in rows], "str"),
            "authority": col([f.get("authority") or "" for f in rows], "str"),
            "layer": col([f.get("layer") or "" for f in rows], "str")}
    keys = sorted({k for f in rows for k in f.get("attrs", {})})
    for k in keys:
        vals = [f.get("attrs", {}).get(k) for f in rows]
        has = [0 if v is None else 1 for v in vals]
        kinds = {type(v) for v in vals if v is not None}
        if kinds <= {int, float}:
            cols[k] = col([float(v) if v is not None else 0.0 for v in vals], "f64")
        elif kinds == {bool}:
            cols[k] = col([bool(v) for v in vals], "bool")
        else:
            out = []
            for v in vals:
                if v is None:
                    out.append("")
                elif isinstance(v, dict) and set(v) == {"at", "name"}:
                    out.append(v["name"])                       # a name attribute: its name
                elif isinstance(v, str):
                    out.append(v)
                else:
                    out.append(json.dumps(v, sort_keys=True, separators=(",", ":")))
            cols[k] = col(out, "str")
        cols[k + "?"] = col(has, "u8")
    return cols


def fields_data(build, man, objects):
    out = {}
    ny, nx = man["frame"]["posts"][1], man["frame"]["posts"][0]
    for name, e in man["fields"].items():
        entry = {"type": e["type"], "role": e["role"], "tags": sorted(e["tags"] or []),
                 "unit": e["unit"], "i0": 0, "j0": 0, "i1": nx, "j1": ny}
        if e["type"] == "weights":
            mats = sorted({m for ch in man["chunks"] for m in ch["weights"].get(name, {}).get("materials", [])})
            arr = np.zeros((len(mats), ny, nx), dtype="|u1")
            for ch in man["chunks"]:
                w = ch["weights"].get(name)
                if not w or not w["file"]:
                    continue
                a = np.load(os.path.join(build, w["file"]))
                i0, j0, i1, j1 = ch["posts"]
                for pl, mname in enumerate(w["materials"]):
                    arr[mats.index(mname), j0:j1 + 1, i0:i1 + 1] = a[pl]
            entry["materials"] = mats
        else:
            arr = np.load(os.path.join(build, e["file"]))
            if "cover" in e:
                continue                                  # a bounded field: not drawn on the map
        entry["blob"], entry["path"] = blob(objects, np.ascontiguousarray(arr))
        out[name] = entry
    return out


def instances_data(build, man):
    ins = man["instances"]
    names = open(os.path.join(build, ins["names"])).read().split("\n")[:-1] if ins["count"] else []
    a = {k: np.load(os.path.join(build, ins[k])).tolist() for k in ("x", "y", "z", "face", "scale", "tint")}
    asset = []
    for ch in man["chunks"]:
        for aid, (first, count) in sorted(ch["instances"].items(), key=lambda t: t[1][0]):
            asset.extend([aid] * count)
    return {"name": col(names, "str"), "asset": col(asset, "str"), "by": col([""] * len(names), "str"),
            "x": col(a["x"], "i32"), "y": col(a["y"], "i32"), "z": col(a["z"], "f64"),
            "face": col(a["face"], "i32"), "scale": col(a["scale"], "f64"), "tint": col(a["tint"], "f64")}


def pack_data(man):
    ids = sorted(man["assets"])
    return {"id": col(ids, "str"), "taxon": col([man["assets"][i]["taxon"] or "" for i in ids], "str"),
            "bounds": col([[int(round(v * 1000)) for v in man["assets"][i]["bounds"]] for i in ids],
                          "i32", [len(ids), 6])}


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("build")
    ap.add_argument("--step", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--python", default=sys.executable)
    a = ap.parse_args(argv)
    with open(a.step, "rb") as f:
        doc = tomllib.load(f)
    ((sid, step),) = doc["step"].items()
    man = json.load(open(os.path.join(a.build, "manifest.json")))
    fr = man["frame"]
    work = tempfile.mkdtemp(prefix="via-mapexport-")
    objects = os.path.join(work, "objects")
    os.makedirs(a.out, exist_ok=True)
    req = {"step": sid, "when": step["when"], "params": step.get("params", {}), "out": os.path.abspath(a.out),
           "objects": objects, "seed": man["seed"],
           "grid": {"extent_mm": [int(round(v * 1000)) for v in fr["extent"]], "nx": fr["posts"][0],
                    "ny": fr["posts"][1], "resolution_mm": int(round(fr["resolution"] * 1000))},
           "data": {"fields": fields_data(a.build, man, objects), "features": features_columns(a.build),
                    "instances": instances_data(a.build, man), "pack": pack_data(man), "inputs": []}}
    rq = os.path.join(work, "request.json")
    with open(rq, "w") as f:
        json.dump(req, f)
    cmd = list(step["command"])
    if cmd[0] == "python":
        cmd[0] = a.python
    cmd = [c.replace("lib:", os.path.join(ROOT, "lib") + "/") for c in cmd]
    env = dict(os.environ, VIA_STEP=rq, PYTHONPATH=os.path.join(ROOT, "sdk", "python"))
    r = subprocess.run(cmd, env=env, stdin=subprocess.DEVNULL)
    import shutil

    shutil.rmtree(work, ignore_errors=True)
    return r.returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
