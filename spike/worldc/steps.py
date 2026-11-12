"""Steps: the points of a build that the user's own code can join.

A `[[step]]` row names a command and a point. Two points exist: `generate`, which writes a layer
before the world is compiled or after one of its layers, and `lint`, which reads a finished build.
The compiler hands a step one file, the request, and takes back files in the folder the request
names. Only plain data crosses: JSON, and rasters of 32-bit floats. The command is the user's, in
any language; `sdk/viasdk` is a convenience over this same contract and nothing more.

Owns running the steps and the record that lets a generate step be skipped when nothing it reads
has changed. What a step computes is not ours: the record covers its inputs, not its result.
"""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

from .diagnostics import Diagnostics

SPIKE = Path(__file__).resolve().parent.parent
CONTRACT = "via-step-0"
POINTS = ("generate", "lint")


def _frame(doc: dict) -> dict:
    w = doc["world"]
    x0, y0, x1, y1 = map(float, w["extent"])
    res = float(w.get("resolution", 0.5))
    frame = {"extent": [x0, y0, x1, y1], "resolution": res, "nx": int(round((x1 - x0) / res)) + 1,
             "ny": int(round((y1 - y0) / res)) + 1, "seed": int(w.get("seed", 0))}
    if w.get("given"):      # built as a part and given something by its master: a different build of the same world
        frame["given"] = w["given"]
    return frame


def _digest_dir(folder: Path) -> str:
    h = hashlib.sha256()
    for p in sorted(folder.rglob("*")):
        if p.is_file() and p.suffix != ".png":
            h.update(p.relative_to(folder).as_posix().encode())
            h.update(p.read_bytes())
    return h.hexdigest()


def _key(step: dict, home: Path, frame: dict, build: Path | None) -> str:
    """Everything the compiler knows a step reads: its command, its parameters, the files it names
    (the script among them), the frame, and the build it is handed. A step that reads anything else
    (another file, the clock, the network) must say so in `inputs`, or its result goes stale unseen."""
    h = hashlib.sha256(json.dumps([CONTRACT, step["run"], step.get("params", {}), frame], sort_keys=True).encode())
    for name in [a for a in step["run"] if (home / a).is_file()] + list(step.get("inputs", [])):
        h.update(name.encode())
        h.update((home / name).read_bytes() if (home / name).is_file() else b"missing")
    if build is not None:
        h.update(_digest_dir(build).encode())
    return h.hexdigest()


def _run(step: dict, home: Path, request: dict, work: Path, diag: Diagnostics) -> bool:
    out = Path(request["out"])
    out.mkdir(parents=True, exist_ok=True)
    work.mkdir(parents=True, exist_ok=True)
    (work / "request.json").write_text(json.dumps(request, indent=1))
    cmd = [sys.executable if a == "python" else a for a in step["run"]]
    env = dict(os.environ, VIA_STEP=str(work / "request.json"),
               PYTHONPATH=os.pathsep.join(filter(None, [str(SPIKE / "sdk"), os.environ.get("PYTHONPATH", "")])))
    started = time.time()
    done = subprocess.run(cmd, cwd=home, env=env, capture_output=True, text=True)
    if done.returncode != 0:
        tail = " | ".join((done.stderr or done.stdout).strip().splitlines()[-3:])
        diag.error("step.failed", step["id"], f"`{' '.join(step['run'])}` ended with status {done.returncode}: {tail}")
        return False
    said = out / "diagnostics.json"
    if said.is_file():
        for item in json.loads(said.read_text()):
            diag.external(step["id"], item)
    print(f"  step {step['id']}: {request['point']} ran in {time.time() - started:.1f} s")
    diag.info("step.ran", step["id"], f"{request['point']} step joined the build")
    return True


def steps_of(doc: dict, diag: Diagnostics) -> list[dict]:
    good = []
    for step in doc.get("step", []):
        if step.get("at") not in POINTS:
            diag.error("step.point", str(step.get("id", "step")), f"`at = \"{step.get('at')}\"` is not a point a step can join",
                       hint="points: " + ", ".join(POINTS))
        else:
            good.append(step)
    return good


def run_generate(path: Path, doc: dict, diag: Diagnostics, compile_lower) -> list[tuple]:
    """Run every generate step in the order written and return the layers they wrote, as
    (id, file, after). `compile_lower(upto, generated, out)` compiles the layers at and below one
    layer, with what earlier steps wrote, so that a step sees the world as it stands beneath it."""
    home, frame = path.parent.resolve(), _frame(doc)
    layer_ids = [l["id"] for l in doc.get("layer", [])]
    generated: list[tuple] = []
    for step in [s for s in steps_of(doc, diag) if s["at"] == "generate"]:
        sid, after = step["id"], step.get("after")
        if after is not None and after not in layer_ids:
            diag.error("step.after", sid, f"it runs after layer '{after}', which the world does not have")
            continue
        kept = sid + ("@" + frame["given"] if "given" in frame else "")      # what it wrote alone is kept apart
        out = home / "gen" / kept
        work = SPIKE / "build" / ".steps" / home.name / kept
        build = None
        if after is not None:
            build = work / "below"
            lower = compile_lower(after, list(generated), build)
            if lower.diag.count("error"):
                diag.adopt(lower.diag, f"below {sid}")
        key = _key(step, home, frame, build)
        layer = out / "layer.json"
        if layer.is_file() and (out / ".key").is_file() and (out / ".key").read_text() == key:
            for item in json.loads((out / "diagnostics.json").read_text()) if (out / "diagnostics.json").is_file() else []:
                diag.external(sid, item)
            print(f"  step {sid}: reused, nothing it reads has changed")
            diag.info("step.ran", sid, "generate step joined the build")
            generated.append((sid, layer, after))
            continue
        for old in out.glob("*") if out.is_dir() else []:      # a stale result must not survive a failed run
            if old.is_file():
                old.unlink()
        request = {"contract": CONTRACT, "point": "generate", "id": sid, "params": step.get("params", {}), "frame": frame,
                   "build": str(build) if build else None, "out": str(out)}
        if not _run(step, home, request, work, diag):
            continue
        if not layer.is_file():
            diag.error("step.no_output", sid, "the step ended without writing layer.json", hint=f"expected {layer}")
            continue
        (out / ".key").write_text(key)
        generated.append((sid, layer, after))
    return generated


def run_lints(path: Path, doc: dict, diag: Diagnostics, build: Path) -> None:
    """Run every lint step on the finished build. A lint is never skipped: it is cheap, and its
    whole worth is that what it says is about the build in front of it."""
    home = path.parent.resolve()
    for step in [s for s in doc.get("step", []) if s.get("at") == "lint"]:
        work = SPIKE / "build" / ".steps" / home.name / step["id"]
        request = {"contract": CONTRACT, "point": "lint", "id": step["id"], "params": step.get("params", {}),
                   "frame": _frame(doc), "build": str(build), "out": str(work / "out")}
        for old in (work / "out").glob("*") if (work / "out").is_dir() else []:
            old.unlink()
        _run(step, home, request, work, diag)
