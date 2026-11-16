"""Writes the pistol's pack from its published numbers (challenge 47; spec/assets.md 12).

    python3.12 examples/pistol/make_pack.py            # writes pack.toml, accessories/*.toml, models/*.glb
    python3.12 examples/pistol/make_pack.py --check    # exits 1 when any of them is not what it writes
    via pack accept examples/pistol/pack.toml          # after writing: pack.facts.toml for the models

Every part's size, height (`sink`) and socket (a mount on the part it rides on) is worked out here
from published.toml, in whole millimetres, so the world names no coordinate: each part stands `on`
a mount of another (world.toml). A pack holds facts, not formulas (R30), so the formulas that the
spike wrote as attributes live in this tool, and its output is committed and checked (--check), as
a step's committed result is (protocol.md, `gen/`). Standard library (tomllib: 3.11+) and
tools/packgen's `glb` writer.

The grip leans (published.toml `grip_lean`); a part stands upright (spec/assets.md 12.5), so a
leaning part is a model: the grip and its panels are written here as glTF models (models/*.glb),
each a box sheared back by the lean, by packgen's writer (glTF's convention: +Y up, front +Z, so
the asset's x is the model's -x and the asset's y the model's z). Their boxes below are their plan
bounds; `LEAN` keeps each one's shear.

The object's frame, millimetres: x from the tail's back (0) to the muzzle (overall_length), y across
(0 on the bore's line), z from the base plate's underside (0) up. A part's pivot is its base centre
(assets.md 6): x0 + length // 2, so a box of odd length keeps its back and front on the lattice.
"""

import math
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "tools" / "packgen"))

import packgen  # noqa: E402

LEAN = {}  # a leaning part: name -> (shear in mm over its height, its length square to the lean)
MODEL = {"grip": "bark", "grip_panels": "timber"}  # packgen material of each model part


def parts(p):
    """Each part: (x0, length, y, width, z0, height, host, colour, taxon). `y` is the pivot's."""
    pub, est, acc = p["published"], p["estimate"], p["accessory"]
    L, H, W = pub["overall_length"], pub["height"], pub["width"]
    slide_h = round(est["slide_h"] * H)
    frame_h = round(est["frame_h"] * H)
    slide_w = round(est["slide_w"] * W)
    tang = round(est["tang"] * L)
    crown, sight_h = est["crown"], est["sight_h"]
    steel, dark, wood, light = [0.22, 0.23, 0.25], [0.16, 0.16, 0.17], [0.36, 0.22, 0.12], [0.85, 0.85, 0.82]
    out = {}
    # the slide and what rides on it
    ls = L - tang - crown
    s_top = H - sight_h
    out["slide"] = (tang, ls, 0, slide_w, s_top - slide_h, slide_h, "frame", steel, "part/pistol")
    # the frame and what is fixed to it
    f_top = s_top - slide_h
    f_bot = f_top - frame_h
    out["frame"] = (tang, round(0.9 * ls), 0, slide_w, f_bot, frame_h, None, dark, "part/pistol")
    out["tail"] = (0, tang + 8, 0, round(0.6 * slide_w), f_bot - 2, 10, "frame", dark, "part/pistol")
    out["hammer"] = (tang - 6, 8, 0, 6, f_top + 1, 13, "frame", dark, "part/pistol")
    grip_l = round(est["grip_l"] * L)
    lean = math.tan(math.radians(est["grip_lean"]))
    grip_c = 3 + grip_l / 2 + lean * f_bot / 2  # the leaning grip's middle
    g0 = 3  # the grip's back at its foot; it leans forward by `shear` to its top, under the frame
    shear = round(lean * f_bot)
    out["grip"] = (g0, grip_l + shear, 0, round(0.9 * slide_w), 0, f_bot, "frame", wood, "part/pistol")
    LEAN["grip"] = (shear, grip_l)
    pl, p0, ph = round(0.82 * grip_l), 8, round(0.78 * f_bot)
    pb = g0 + (grip_l - pl) // 2 + round(lean * p0)  # the panels' back at their foot
    ps = round(lean * ph)
    out["grip_panels"] = (pb, pl + ps, 0, W, p0, ph, "frame", wood, "part/pistol")
    LEAN["grip_panels"] = (ps, pl)
    out["base_plate"] = (g0 - 1, grip_l + 3, 0, round(0.9 * slide_w), 0, 4, "frame", dark, "part/pistol")
    r = round(est["guard_r"] * L)
    gx = round(grip_c + lean * f_bot / 2 + grip_l / 2 + r - 2.5)
    gz = round(f_bot - 0.55 * r)
    # the guard's bottom bar runs back into the grip's front at its height, so the guard is closed
    gb = round(g0 + grip_l + lean * (gz - r - 3)) - 1
    out["guard_bottom"] = (gb, gx + r - gb, 0, 10, gz - r - 3, 6, "frame", dark, "part/pistol")
    out["guard_front"] = (gx + r - 3, 6, 0, 10, gz - r - 3, f_bot - (gz - r - 3), "frame", dark, "part/pistol")
    out["trigger"] = (gx - 8, 6, 0, 6, gz - 6, 20, "frame", light, "part/pistol")
    d = round(pub["bore"] + 4)
    b_mid = (s_top + f_top) // 2
    out["barrel"] = (L - pub["barrel_length"], pub["barrel_length"], 0, d, b_mid - d // 2, d, "frame", steel, "part/pistol")
    # the sights: the front one's pivot is sight_radius ahead of the rear one's
    fs_c = tang + ls - 9
    out["front_sight"] = (fs_c - 3, 7, 0, 3, s_top, sight_h, "slide", light, "part/pistol")
    rs_c = fs_c - pub["sight_radius"]
    out["rear_left"] = (rs_c - 4, 8, 5, 6, s_top, sight_h, "slide", light, "part/pistol")
    out["rear_right"] = (rs_c - 4, 8, -5, 6, s_top, sight_h, "slide", light, "part/pistol")
    # sockets: where an accessory's own frame is put (x, y, host); its height is its parts' sink
    sockets = {"optic": (rs_c + 26, 0, "slide"), "muzzle": (L, 0, "barrel"), "rail": (tang + round(0.9 * ls) - 24, 0, "frame")}
    # the accessories' parts, each in its own frame from its socket (accessories/*.toml)
    ox, mx, rx = sockets["optic"][0], L, sockets["rail"][0]
    acc_parts = {
        "optic_base": (ox - acc["optic_l"] // 2, acc["optic_l"], 0, slide_w, s_top, acc["optic_h"], None, dark, "accessory/optic"),
        # the hood is a frame round its window: two posts and a top bar (a hole is the gap between
        # parts, spec/assets.md 12.5)
        "optic_post_l": (ox + acc["optic_l"] // 2 - 4, 4, (slide_w - 3) // 2, 3, s_top + acc["optic_h"], acc["hood_h"], None, dark, "accessory/optic"),
        "optic_post_r": (ox + acc["optic_l"] // 2 - 4, 4, -((slide_w - 3) // 2), 3, s_top + acc["optic_h"], acc["hood_h"], None, dark, "accessory/optic"),
        "optic_top": (ox + acc["optic_l"] // 2 - 4, 4, 0, slide_w, s_top + acc["optic_h"] + acc["hood_h"] - 4, 4, None, dark, "accessory/optic"),
        "can": (mx, acc["can_l"], 0, acc["can_d"], b_mid - acc["can_d"] // 2, acc["can_d"], None, dark, "accessory/muzzle"),
        "light_body": (rx - acc["light_l"] // 2, acc["light_l"], 0, slide_w - 1, f_bot - acc["light_h"], acc["light_h"], None, dark, "accessory/light"),
        "light_head": (rx + acc["light_l"] // 2 - 1, 10, 0, 21, f_bot - acc["light_h"], 21, None, light, "accessory/light"),
    }
    return out, sockets, acc_parts


def pivot(part):
    x0, length, y = part[0], part[1], part[2]
    return x0 + length // 2, y


def mm(v):
    s = f"{v / 1000:.3f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def write(p):
    out, sockets, acc_parts = parts(p)
    travel = p["state"]["racked"]["travel"]
    lines = [
        "# Written by make_pack.py from published.toml. Do not edit: change the numbers and run it again",
        "# (`--check` tells a stale pack). The pistol's parts and its accessories' parts: primitives, and",
        "# the leaning grip and its panels as models (models/, a part stands upright: assets.md 12.5).",
        "# Sizes [x, y, z] metres, x along the length toward the muzzle; `sink` (negative) is how far a",
        "# part's underside stands above the base plate's (assets.md 12.2); each mount is a socket where",
        "# another part, or an accessory's frame, is put (assets.md 12.1).",
        "",
        'root = "models"',
        "",
        '[taxon."part/pistol"]                   # a thing on a bench claims no ground: parts nest inside each other',
        'defaults = { solidity = "none", collision = "box" }',
        '[taxon."accessory/optic"]',
        'defaults = { solidity = "none", collision = "box" }',
        '[taxon."accessory/muzzle"]',
        'defaults = { solidity = "none", collision = "box" }',
        '[taxon."accessory/light"]',
        'defaults = { solidity = "none", collision = "box" }',
        "",
        "[material.bench]",
        "colour = [0.62, 0.6, 0.55]",
    ]
    every = dict(out) | acc_parts
    for name, (x0, length, y, width, z0, height, host, colour, taxon) in every.items():
        if name in LEAN:
            lines += ["", f"[asset.{name}]", f'file      = "{name}.glb"', f'taxon     = "{taxon}"']
        else:
            lines += [
                "",
                f"[asset.{name}]",
                'primitive = "box"',
                f"size      = [{mm(length)}, {mm(width)}, {mm(height)}]",
                f'taxon     = "{taxon}"',
            ]
        if z0:
            lines.append(f"sink      = {mm(-z0)}")
        if name not in LEAN:
            lines.append(f"colour    = [{', '.join(str(c) for c in colour)}]")
        hx, hy = pivot(every[name])
        mounts = [(k, pivot(v)) for k, v in out.items() if v[6] == name]
        if name == "frame":
            sx, sy = pivot(out["slide"])
            mounts.append(("slide_racked", (sx - travel, sy)))
        mounts += [(k, (sx_, sy_)) for k, (sx_, sy_, h) in sockets.items() if h == name]
        for k, (mx_, my_) in mounts:
            lines += ["", f"[asset.{name}.mount.{k}]", f"at = [{mm(mx_ - hx)}, {mm(my_ - hy)}]"]
    return "\n".join(lines) + "\n", accessories(sockets, acc_parts), models(out)


def models(out):
    """Each leaning part as a glTF model: a box of its height whose top stands `shear` mm ahead of
    its foot, centred in plan on its pivot (so no pivot moves, assets.md 2.3), its foot at 0."""
    files = {}
    for name, (shear, length) in LEAN.items():
        x0, total, _, width, _, height, *_ = out[name]
        hx = x0 + total // 2  # the pivot, as pivot() puts it
        a, b, h = (x0 - hx) / 1000, (x0 + length - hx) / 1000, height / 1000
        w0, w1 = -(width // 2) / 1000, (width - width // 2) / 1000  # on the lattice, as a box's
        s = shear / 1000
        # asset (x, y, z) is model (-x, z, y); the shear moves the top toward asset +x, model -x
        box = packgen.box(-b, 0.0, w0, -a, h, w1)
        tris = [tuple((v[0] - s * v[1] / h, v[1], v[2]) for v in tri) for tri in box]
        files[f"models/{name}.glb"] = packgen.glb([(MODEL[name], tris)])
    return files


ACCESSORY = {"optic": ("optic", "a reflex sight on the slide"), "suppressor": ("muzzle", "a muzzle device on the barrel"),
             "light": ("rail", "a light under the frame's rail")}
HOST = {"frame": "pistol", "slide": "slide", "barrel": "barrel"}


def accessories(sockets, acc_parts):
    """Each accessory overlay: a set of its parts in its own frame, placed on its socket. It names
    the socket only; nothing of the pistol is written again (spec/assets.md 12.3)."""
    files = {}
    prefix = {"optic": "optic_", "suppressor": "can", "light": "light_"}
    for acc, (socket, what) in ACCESSORY.items():
        sx, sy, host = sockets[socket]
        lines = [
            "# Written by make_pack.py. An accessory overlay: " + what + ", attached to the socket",
            f"# `{HOST[host]}.{socket}` (spec/assets.md 12.3). Its parts are a set in its own frame, whose origin is",
            "# the socket; the host is not written again. Build with --change accessories/" + acc + ".toml.",
        ]
        for name, part in acc_parts.items():
            if name.startswith(prefix[acc]):
                px, py = pivot(part)
                lines += ["", f"[set.{acc}.item.{name}]", f'asset = "{name}"', f"at    = [{mm(px - sx)}, {mm(py - sy)}]"]
        lines += ["", f"[instance.{acc}]", f'set = "{acc}"', f'on  = {{ ref = "{HOST[host]}.{socket}" }}']
        files[f"accessories/{acc}.toml"] = "\n".join(lines) + "\n"
    return files


def main():
    p = tomllib.loads((HERE / "published.toml").read_text())
    text, acc, glbs = write(p)
    want = {"pack.toml": text} | acc | glbs
    if "--check" in sys.argv[1:]:
        def same(k, t):
            f = HERE / k
            return f.exists() and (f.read_bytes() if isinstance(t, bytes) else f.read_text()) == t

        stale = [k for k, t in want.items() if not same(k, t)]
        if stale:
            print("stale, run make_pack.py: " + ", ".join(stale), file=sys.stderr)
            sys.exit(1)
        print("pack.toml, accessories/ and models/ are what make_pack.py writes")
        return
    for d in ("accessories", "models"):
        (HERE / d).mkdir(exist_ok=True)
    for k, t in want.items():
        (HERE / k).write_bytes(t) if isinstance(t, bytes) else (HERE / k).write_text(t)
        print(f"wrote {k}")


if __name__ == "__main__":
    main()
