"""The demo's sweep and hero (docs/design/demo.md): the high street's path moved in even steps, each
step a real build on one store, each drawn from one camera by the showcase (engines/godot), each
measured against the base. Then the frames are lettered and joined (compose.py), and the hero
picture is composed from the base, one step and their numbers.

    python3 examples/demo/sweep.py OUT [--from 125] [--to 165] [--step 5] [--base 125] [--hero 165]

Needs the release `via` (target/release/via), Godot at $GODOT (the pinned release's macOS install
by default), `sdk/python/.venv` for the steps and the measures, and Kenney's models fetched
(examples/borrowed/fetch.py). Writes under OUT: `changes/` (one change file a step: the one line),
`builds/` (each build, on one store), `frames/` (each step drawn), `measures/` (each step's
measures against the base, and its build's run totals), `sweep.json` (what was run, and with what),
then compose.py's output. Standard library only; compose.py needs PIL and ffmpeg.
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
# One camera for every frame: from over the stream's far bank, south-east of the town, across it
# to the wooded hill (world metres). Ours, chosen by looking.
EYE, AT, FOV = (350, 25, 105), (235, 170, 10), 46
SUN = (255, 18)                      # bearing, elevation: a low sun from the west, long shadows
SIZE = (1920, 1080)
LINE = "path = [[45, {y}], [435, {y}]]"


def run(cmd, log=None, env=None):
    t0 = time.time()
    with open(log, "w") if log else open(os.devnull, "w") as f:
        r = subprocess.run([str(c) for c in cmd], cwd=ROOT, stdout=f, stderr=subprocess.STDOUT, env=env)
    return r.returncode, time.time() - t0


def build(out, name, change=None):
    env = dict(os.environ, VIA_LIB=str(ROOT / "lib"))
    ch = ["--change", change] if change else []
    run([VIA, "trust", WORLD, *ch, "--yes"], env=env)
    code, _ = run([VIA, "build", WORLD, *ch, "--out", out / "builds" / name, "--store", out / "store"],
                  log=out / "builds" / f"{name}.log", env=env)
    if code != 0:
        sys.exit(f"sweep: the build `{name}` failed; see {out / 'builds' / (name + '.log')}")


def draw(out, name, eye=EYE, at=AT, size=SIZE, file=None, log=None):
    code, _ = run([GODOT, "--path", "engines/godot", "--rendering-method", "forward_plus", "--audio-driver", "Dummy",
                   "res://showcase.tscn", "--", "--build", out / "builds" / name, "--pack", ROOT,
                   "--out", file or out / "frames" / f"{name}.png", "--eye", ",".join(map(str, eye)),
                   "--at", ",".join(map(str, at)), "--fov", str(FOV), "--size", f"{size[0]},{size[1]}",
                   "--sun", f"{SUN[0]},{SUN[1]}", "--option", "water=tag:water_level"],
                  log=out / "frames" / f"{log or name}.log")
    if code != 0:
        sys.exit(f"sweep: drawing `{name}` failed; see {out / 'frames' / ((log or name) + '.log')}")


def measure(out, name, base="base"):
    b = out / "builds"
    args = [PY, "examples/demo/measure.py", b / name, "--run", b / f"{name}.run.json"]
    if name != base:
        args += ["--base", b / base]
    else:
        args += ["--world", WORLD]
    r = subprocess.run([str(a) for a in args], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"sweep: measuring `{name}` failed: {r.stderr}")
    (out / "measures" / f"{name}.json").write_text(r.stdout)
    return json.loads(r.stdout)


def main(argv):
    args = argv[1:]
    opt = {"--from": "125", "--to": "165", "--step": "5", "--base": "125", "--hero": "165"}
    for k in list(opt):
        if k in args:
            i = args.index(k)
            opt[k] = args[i + 1]
            del args[i:i + 2]
    if len(args) != 1:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    out = Path(args[0]).resolve()
    for d in ("changes", "builds", "frames", "measures"):
        (out / d).mkdir(parents=True, exist_ok=True)
    lo, hi, step, base_y, hero_y = (int(opt[k]) for k in ("--from", "--to", "--step", "--base", "--hero"))
    ys = list(range(lo, hi + 1, step))
    if base_y not in ys or hero_y not in ys:
        sys.exit("sweep: --base and --hero must be steps of the sweep")
    print(f"sweep: base, then {len(ys) - 1} steps of the high street's line", flush=True)
    build(out, "base")
    steps = []
    # The hero's step is built straight after the base, so its numbers (what ran, what changed) are
    # the ones the README's two commands give (D's review: built after the step before it, it ran
    # less); the others follow in order.
    order = [base_y, hero_y] + [y for y in ys if y not in (base_y, hero_y)]
    for y in order:
        name = "base" if y == base_y else f"y{y}"
        if y != base_y:
            ch = out / "changes" / f"{name}.toml"
            ch.write_text("# The one line the demo changes (docs/design/demo.md): the high street's path.\n"
                          f"[feature.high_street]\n{LINE.format(y=y)}\n")
            build(out, name, ch)
        draw(out, name)
        m = measure(out, name)
        steps.append({"y": y, "name": name, "line": LINE.format(y=y), "measures": m})
        print(f"sweep: y = {y}: {m.get('buildings')} houses, {m.get('trees')} trees, {m.get('warnings')} warnings, "
              f"{(m.get('build_ms') or 0) / 1000:.0f} s", flush=True)
    # The hero: the base and one step, large, from the same camera.
    draw(out, "base", size=(2400, 1350), file=out / "frames" / "hero_before.png", log="hero_before")
    draw(out, f"y{hero_y}", size=(2400, 1350), file=out / "frames" / "hero_after.png", log="hero_after")
    godot = subprocess.run([GODOT, "--version"], capture_output=True, text=True).stdout.strip()
    rev = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    (out / "sweep.json").write_text(json.dumps({
        "world": WORLD, "line": LINE, "base": base_y, "hero": hero_y, "steps": steps,
        "camera": {"eye": EYE, "at": AT, "fov": FOV, "sun": SUN, "size": SIZE},
        "godot": godot, "head": rev, "order": order,
        "store": "one, every build on it in `order`: the base, the hero's step, the others; none --against",
    }, indent=1))
    print(f"sweep: {out / 'sweep.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
