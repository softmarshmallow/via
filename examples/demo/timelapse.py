"""The demo's timelapse and gallery (docs/design/demo.md, "The knobs"): the world's knobs turned one
after another while the camera circles the valley, and each knob turned alone beside the base.

    python3 examples/demo/timelapse.py OUT [--frames-per-state 20] [--fade 8] [--redraw]

- **The timelapse.** State 0 is the base; state k is the base with the first k knobs of KNOBS
  turned (`via build --change` once per knob, so every state is the base and its knobs' own change
  files, nothing else), each built on one store. Each state is drawn on its own arc of one orbit
  (engines/godot/showcase.gd `--orbit`), and drawn on past its arc's end by `--fade` frames, which
  the next state's first frames are blended with: the camera never stops, and the map changes under
  it.
- **The gallery.** Each knob turned alone, built on the same store, drawn from one fixed eye; the
  base beside them.
- **The numbers.** Every state and every knob measured against the base by measure.py.

Writes under OUT: `builds/`, `frames/`, `measures/`, `timelapse.json`. compose.py then letters and
joins them. Needs what sweep.py needs. Standard library only.
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORLD = "examples/demo/world.toml"
VIA = ROOT / "target/release/via"
PY = ROOT / "sdk/python/.venv/bin/python"
GODOT = os.environ.get("GODOT", "/Applications/Godot.app/Contents/MacOS/Godot")
# The knobs, in the order the timelapse turns them: (change file, what it says). Each change file
# is one line under its table (examples/demo/changes/).
KNOBS = [
    ("river", "the stream, 22 m across"),
    ("north", "the high street, 40 m north"),
    ("grand", "the houses, all three blocks wide"),
    ("city", "the town, four to six storeys"),
    ("hills", "the land, rolling hills"),
    ("forest", "the woods close in"),
    ("pines", "the woods, all pines"),
    ("seed", "the seed, drawn again"),
]
# The gallery: each knob alone, and one the timelapse does not turn.
GALLERY = KNOBS + [("farmland", "the woods cleared")]
# The orbit: round (CX, CY) looking at CZ, RADIUS out, HEIGHT up; bearings FROM to TO over the
# whole timelapse (clockwise from north: from the south-east, round the south, to the south-west).
ORBIT = (230, 165, 15, 160, 95)
ARC = (122, 238)        # inside the world's extent all the way, its far edge out of frame (D's review)
GALLERY_EYE, GALLERY_AT = (350, 25, 105), (235, 170, 10)
FOV, SUN = 46, (255, 18)
SIZE = (1920, 1080)


def run(cmd, log):
    t0 = time.time()
    with open(log, "w") as f:
        r = subprocess.run([str(c) for c in cmd], cwd=ROOT, stdout=f, stderr=subprocess.STDOUT,
                           env=dict(os.environ, VIA_LIB=str(ROOT / "lib")))
    return r.returncode, time.time() - t0


def build(out, name, knobs):
    ch = [a for k in knobs for a in ("--change", f"examples/demo/changes/{k}.toml")]
    run([VIA, "trust", WORLD, *ch, "--yes"], out / "builds" / f"{name}.trust.log")
    code, _ = run([VIA, "build", WORLD, *ch, "--out", out / "builds" / name, "--store", out / "store"],
                  out / "builds" / f"{name}.log")
    if code != 0:
        sys.exit(f"timelapse: the build `{name}` failed; see {out / 'builds' / (name + '.log')}")


def godot(out, name, args, log):
    code, _ = run([GODOT, "--path", "engines/godot", "--rendering-method", "forward_plus", "--audio-driver", "Dummy",
                   "res://showcase.tscn", "--", "--build", out / "builds" / name, "--pack", ROOT,
                   "--fov", FOV, "--sun", f"{SUN[0]},{SUN[1]}", "--option", "water=tag:water_level", *args],
                  out / "frames" / f"{log}.log")
    if code != 0:
        sys.exit(f"timelapse: drawing `{name}` failed; see {out / 'frames' / (log + '.log')}")


def measure(out, name):
    b = out / "builds"
    args = [PY, "examples/demo/measure.py", b / name, "--run", b / f"{name}.run.json"]
    args += ["--world", WORLD] if name == "base" else ["--base", b / "base"]
    r = subprocess.run([str(a) for a in args], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"timelapse: measuring `{name}` failed: {r.stderr}")
    (out / "measures" / f"{name}.json").write_text(r.stdout)
    return json.loads(r.stdout)


def line_of(knob):
    """The change file's one line, as the frames letter it: its table and its key."""
    text = (ROOT / f"examples/demo/changes/{knob}.toml").read_text().splitlines()
    body = [t for t in text if t.strip() and not t.startswith("#")]
    return body[0] if body[0].startswith("[") and len(body) == 2 else None, body[-1]


def main(argv):
    args = argv[1:]
    opt = {"--frames-per-state": "20", "--fade": "8"}
    redraw = "--redraw" in args                      # draw again from the builds already in OUT
    if redraw:
        args.remove("--redraw")
    for k in list(opt):
        if k in args:
            i = args.index(k)
            opt[k] = args[i + 1]
            del args[i:i + 2]
    if len(args) != 1:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    out = Path(args[0]).resolve()
    for d in ("builds", "frames", "measures"):
        (out / d).mkdir(parents=True, exist_ok=True)
    per, fade = int(opt["--frames-per-state"]), int(opt["--fade"])
    n = len(KNOBS) + 1
    step = (ARC[1] - ARC[0]) / (n * per)                  # degrees a frame
    states = []
    for k in range(n):
        name = "base" if k == 0 else f"s{k}"
        if not redraw:
            build(out, name, [x for x, _ in KNOBS[:k]])
        a0 = ARC[0] + k * per * step
        count = per + (fade if k < n - 1 else 0)
        godot(out, name, ["--out", out / "frames" / f"{name}_%03d.png", "--size", f"{SIZE[0]},{SIZE[1]}",
                          "--orbit", ",".join(str(v) for v in (*ORBIT, a0, a0 + count * step, count))], name)
        m = measure(out, name)
        table, line = line_of(KNOBS[k - 1][0]) if k else (None, None)
        states.append({"name": name, "knob": KNOBS[k - 1][0] if k else None, "says": KNOBS[k - 1][1] if k else None,
                       "table": table, "line": line, "frames": count, "measures": m})
        print(f"timelapse: {name}: {m.get('buildings')} houses, {m.get('trees')} trees, {m.get('warnings')} warnings, "
              f"{(m.get('build_ms') or 0) / 1000:.0f} s", flush=True)
    # The gallery: each knob alone, on the same store, from one eye; and the base.
    gallery = []
    eye = ["--eye", ",".join(map(str, GALLERY_EYE)), "--at", ",".join(map(str, GALLERY_AT)), "--size", "1280,720"]
    godot(out, "base", [*eye, "--out", out / "frames" / "g_base.png"], "g_base")
    for knob, says in GALLERY:
        name = f"k_{knob}"
        if not redraw:
            build(out, name, [knob])
        godot(out, name, [*eye, "--out", out / "frames" / f"g_{knob}.png"], f"g_{knob}")
        table, line = line_of(knob)
        gallery.append({"knob": knob, "says": says, "table": table, "line": line, "measures": measure(out, name)})
        print(f"timelapse: gallery {knob}", flush=True)
    rev = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    ver = subprocess.run([GODOT, "--version"], capture_output=True, text=True).stdout.strip()
    (out / "timelapse.json").write_text(json.dumps({
        "world": WORLD, "states": states, "gallery": gallery, "fade": fade, "per": per,
        "orbit": {"centre": ORBIT[:3], "radius": ORBIT[3], "height": ORBIT[4], "arc": ARC, "fov": FOV, "sun": SUN},
        "gallery_eye": {"eye": GALLERY_EYE, "at": GALLERY_AT}, "godot": ver, "head": rev,
        "store": "one: the states in order, then each knob alone; none --against",
    }, indent=1))
    print(f"timelapse: {out / 'timelapse.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
