"""Measures a sidearm build against its own numbers, per state (challenge 46; docs/plan.md M5).

    python3.12 examples/sidearm/measure.py BUILD_DIR [--check STATE]

Prints JSON; with --check, each row of expect.toml's section STATE (`rest`, `racked`) must hold
(`measure`, `band`) and the build's own state must be STATE, else exit 1. Reads only the build directory (spec/build.md): instances/ (names.txt; x.npy, y.npy
int32 mm; z.npy float32 m; face.npy int32 millidegrees; scale.npy) and manifest.json (`assets`:
`bounds`, `taxon`; `chunks` for each row's asset), diagnostics.json (`counts`), and numbers.toml
and expect.toml beside this file. Standard library only.

Protocol (ours, for this world):

- **The parts** are the instances under `sidearm.` (the set placement of world.toml); the frame is
  `sidearm.frame`, and every measure is in its axes (its origin, turned back by its face).
- **The state is read from the build, not told** (spec/assets.md 12.4). The slide's offset is
  `sidearm.upper.slide`'s origin less the frame's, along the frame's x, in mm. `rest` when it is 0
  (the slide sits over the frame, world.toml), `racked` when it is `-travel`, else `unknown`.
- **Size.** Every part's box is its asset's `bounds` scaled, turned by its face (quarter turns
  only), at its origin and `z`. `length`, `width`, `height` are the union's extent along the frame's
  x, y and z, in mm, rounded to the nearest. The leaning grip is a model: its bounds are the
  sheared box's (models/grip.glb).
- **Against its own numbers.** In `rest`, `length`, `width`, `height` must equal numbers.toml's
  exactly (`off_*` 0). In `racked` the slide's back, the rearmost part at rest, moves `travel`
  back, so `expect_length` is `length + travel`; height and width as at rest.
- **Crown.** `crown`: the barrel's front less the slide's front, along the frame's x, in mm. At
  rest it must equal numbers.toml's `crown` (2: the barrel does not show past the slide); racked,
  `crown + travel`.
- **Sight radius.** The distance in mm between the front sight's origin and the mid point of the
  two rear posts' origins, against `sight_radius`, in every state.
"""

import json
import math
import struct
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = "sidearm"


def npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    code = {"'<i4'": "i", "'<f4'": "f", "'<f8'": "d"}[head.split("'descr': ")[1].split(",")[0]]
    n = int(head.split("'shape': (")[1].split(",")[0])
    return list(struct.unpack(f"<{n}{code}", raw[10 + hlen:]))


def turn(ax, ay, f):
    """A point in an asset frame to the world, at bearing f millidegrees (numbers.md 5.1)."""
    s, c = math.sin(math.radians(f / 1000)), math.cos(math.radians(f / 1000))
    return ax * c + ay * s, -ax * s + ay * c


def measure(build):
    build = Path(build)
    man = json.loads((build / "manifest.json").read_text())
    assets = man["assets"]
    inst = build / "instances"
    names = (inst / "names.txt").read_text().splitlines()
    xs, ys, zs = npy(inst / "x.npy"), npy(inst / "y.npy"), npy(inst / "z.npy")
    fs, ks = npy(inst / "face.npy"), npy(inst / "scale.npy")
    by = {n: k for k, n in enumerate(names)}
    asset_of = {}  # a chunk's `instances` maps an asset to [first row, count] (build.md 8)
    for chunk in man["chunks"]:
        for a, (first, count) in chunk["instances"].items():
            for k in range(first, first + count):
                asset_of[names[k]] = a
    num = tomllib.loads((HERE / "numbers.toml").read_text())
    n, travel = num["numbers"], num["state"]["racked"]["travel"]
    k0 = by[f"{ROOT}.frame"]
    fx, fy, ff = xs[k0], ys[k0], fs[k0]

    def local(k):
        """Instance k's origin in the frame's axes, mm."""
        return turn(xs[k] - fx, ys[k] - fy, -ff)

    def extent(k):
        """Instance k's box along the frame's x, y and z: (lo, hi) each, mm."""
        b = [v * 1000 * ks[k] for v in assets[asset_of[names[k]]]["bounds"]]
        lx, ly = local(k)
        pts = [turn(cx, cy, fs[k] - ff) for cx in (b[0], b[3]) for cy in (b[1], b[4])]
        xs_ = [lx + p[0] for p in pts]
        ys_ = [ly + p[1] for p in pts]
        zs_ = [zs[k] * 1000 + b[2], zs[k] * 1000 + b[5]]
        return (min(xs_), max(xs_)), (min(ys_), max(ys_)), (min(zs_), max(zs_))

    parts = [name for name in names if name.startswith(f"{ROOT}.")]
    out = {"parts": len(parts)}
    out["errors"] = json.loads((build / "diagnostics.json").read_text())["counts"]["error"]
    slide, barrel = by[f"{ROOT}.upper.slide"], by[f"{ROOT}.barrel"]
    offset = round(local(slide)[0])
    out["slide_offset"] = offset
    out["state"] = "rest" if offset == 0 else "racked" if offset == -travel else "unknown"
    lo, hi = [math.inf] * 3, [-math.inf] * 3
    for name in parts:
        for i, (a, b) in enumerate(extent(by[name])):
            lo[i], hi[i] = min(lo[i], a), max(hi[i], b)
    out["length"], out["width"], out["height"] = (round(hi[i] - lo[i]) for i in range(3))
    racked = out["state"] == "racked"
    out["expect_length"] = n["length"] + (travel if racked else 0)
    out["off_length"] = out["length"] - out["expect_length"]
    out["off_width"] = out["width"] - n["width"]
    out["off_height"] = out["height"] - n["height"]
    out["crown"] = round(extent(barrel)[0][1] - extent(slide)[0][1])
    out["expect_crown"] = n["crown"] + (travel if racked else 0)
    out["off_crown"] = out["crown"] - out["expect_crown"]
    rl, rr, fsg = (by[f"{ROOT}.upper.{p}"] for p in ("rear_left", "rear_right", "front_sight"))
    mx, my = (xs[rl] + xs[rr]) / 2, (ys[rl] + ys[rr]) / 2
    out["sight_radius"] = round(math.hypot(xs[fsg] - mx, ys[fsg] - my))
    out["off_sight_radius"] = out["sight_radius"] - n["sight_radius"]
    return out


def check(out, state):
    """expect.toml's section `state`: each row's measure within its band, and the build's own state
    that one. Prints a line a row; returns the exit code."""
    rows = tomllib.loads((HERE / "expect.toml").read_text())[state]
    ok = out["state"] == state
    print(f"{'ok  ' if ok else 'FAIL'} {state}.state: state = {out['state']}, checked as {state}")
    bad = not ok
    for name, row in rows.items():
        v = out.get(row["measure"])
        lo, hi = row["band"]
        ok = v is not None and lo <= v <= hi
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {state}.{name}: {row['measure']} = {v}, band [{lo}, {hi}]")
    return 1 if bad else 0


def main():
    args = sys.argv[1:]
    out = measure(args[0])
    if "--check" in args:
        sys.exit(check(out, args[args.index("--check") + 1]))
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
