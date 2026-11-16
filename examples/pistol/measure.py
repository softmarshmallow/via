"""Measures a pistol build against its published numbers, per state (challenge 47; docs/plan.md M5).

    python3.12 examples/pistol/measure.py BUILD_DIR [--check STATE]

Prints JSON. Reads only the build directory (spec/build.md): instances/ (names.txt; x.npy, y.npy
int32 mm; z.npy float32 m; face.npy int32 millidegrees; scale.npy), manifest.json (`assets`:
`bounds`, `taxon`, `mounts`; `chunks` for each row's asset) and diagnostics.json (`counts`), and
published.toml beside this file. Standard library only.

STATE is `rest` or `racked`, then `+<accessory>` for each accessory fitted, in the order optic,
suppressor, light (`racked+optic`): the build's own state and accessories must be those, and each
row of expect.toml's section STATE must hold (`measure`, `band`). Exit 1 when one does not.

Protocol (ours, for this world):

- **The state is read from the build, not told** (spec/assets.md 12.4). `racked` when the `slide`
  instance's origin is the frame's `slide_racked` socket (frame origin plus the mount's `at` turned
  by the frame's face, to the millimetre), `rest` when it is the `slide` socket, else `unknown`.
  The accessories are the instances whose asset's taxon starts with `accessory/`; `fitted` names
  the overlays they come from (an accessory's parts are items of a set placement named for it,
  `optic.optic_base`), in the order optic, suppressor, light.
- **Size.** Every part's box is its asset's `bounds` scaled, turned by its face (quarter turns only:
  the parts stand at the frame's face), at its origin and `z`. `length`, `width`, `height` are the
  union's extent along the frame's x, y and z, in mm, rounded to the nearest. The leaning grip and
  its panels are models (make_pack.py): their bounds are the sheared boxes'. The pistol alone
  (taxon `part/`), and with its accessories (`length_all`, `height_all`, `width_all`).
- **Against the published numbers.** In `rest`, the pistol alone must equal `overall_length`,
  `width` and `height` exactly (`off_*` 0). In `racked`, the slide stands `travel` behind the rest
  socket, so the length grows by what the slide's back passes the tail's: `expect_length` is
  `overall_length + max(0, travel - tang)`, height and width as published. Accessories never count
  against the published numbers: they are reported as `added_length`, `added_height`, `added_width`.
- **Sight radius.** `sight_radius`: the distance in mm between the front sight's origin and the mid
  point of the two rear posts' origins, against published `sight_radius`.
"""

import json
import math
import struct
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ORDER = ("optic", "suppressor", "light")  # the accessory overlays, accessories/<name>.toml


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
    pub = tomllib.loads((HERE / "published.toml").read_text())
    p, est = pub["published"], pub["estimate"]
    travel = pub["state"]["racked"]["travel"]
    tang = round(est["tang"] * p["overall_length"])
    k0 = by["pistol"]
    fx, fy, ff = xs[k0], ys[k0], fs[k0]
    out = {"parts": len(names)}

    def socket(name):
        ax, ay = (v * 1000 for v in assets[asset_of["pistol"]]["mounts"][name]["at"])
        dx, dy = turn(ax, ay, ff)
        return round(fx + dx), round(fy + dy)

    sl = (xs[by["slide"]], ys[by["slide"]])
    out["state"] = "racked" if sl == socket("slide_racked") else "rest" if sl == socket("slide") else "unknown"
    acc = [n for n in names if assets[asset_of[n]]["taxon"].startswith("accessory/")]
    out["accessories"] = sorted({asset_of[n] for n in acc})
    out["fitted"] = [a for a in ORDER if any(n.split(".")[0] == a for n in acc)]
    out["errors"] = json.loads((build / "diagnostics.json").read_text())["counts"]["error"]

    def box(names_in):
        lo, hi = [math.inf] * 3, [-math.inf] * 3
        for n in names_in:
            k = by[n]
            b = [v * 1000 * ks[k] for v in assets[asset_of[n]]["bounds"]]
            for cx in (b[0], b[3]):
                for cy in (b[1], b[4]):
                    dx, dy = turn(cx, cy, fs[k] - ff)  # in the frame's axes
                    wx, wy = xs[k] - fx, ys[k] - fy
                    lx, ly = turn(wx, wy, -ff)
                    for i, v in enumerate((lx + dx, ly + dy)):
                        lo[i], hi[i] = min(lo[i], v), max(hi[i], v)
            for v in (zs[k] * 1000 + b[2], zs[k] * 1000 + b[5]):
                lo[2], hi[2] = min(lo[2], v), max(hi[2], v)
        return [round(hi[i] - lo[i]) for i in range(3)]

    own = [n for n in names if assets[asset_of[n]]["taxon"].startswith("part/")]
    out["length"], out["width"], out["height"] = box(own)
    out["length_all"], out["width_all"], out["height_all"] = box(names)
    expect = p["overall_length"] + (max(0, travel - tang) if out["state"] == "racked" else 0)
    out["expect_length"] = expect
    out["off_length"] = out["length"] - expect
    out["off_width"] = out["width"] - p["width"]
    out["off_height"] = out["height"] - p["height"]
    out["added_length"] = out["length_all"] - out["length"]
    out["added_height"] = out["height_all"] - out["height"]
    out["added_width"] = out["width_all"] - out["width"]
    rl, rr, fsg = by["rear_left"], by["rear_right"], by["front_sight"]
    mx, my = (xs[rl] + xs[rr]) / 2, (ys[rl] + ys[rr]) / 2
    out["sight_radius"] = round(math.hypot(xs[fsg] - mx, ys[fsg] - my))
    out["off_sight_radius"] = out["sight_radius"] - p["sight_radius"]
    return out


def check(out, state):
    """expect.toml's section `state`: each row's measure within its band, and the build's own
    state and accessories those the section names. Prints a line a row; returns the exit code."""
    rows = tomllib.loads((HERE / "expect.toml").read_text())[state]
    want = state.split("+")
    bad = 0
    for name, ok, line in [("state", out["state"] == want[0], f"state = {out['state']}, checked as {want[0]}"),
                           ("fitted", out["fitted"] == want[1:], f"fitted = {out['fitted']}, checked as {want[1:]}")]:
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {state}.{name}: {line}")
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
