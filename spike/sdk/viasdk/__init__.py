"""viasdk: join a build from Python.

    from viasdk import step
    s = step()                              # the request the compiler handed this run
    h = s.build.field("height")             # what stands beneath this step, as a numpy array
    s.write_field("height", h_new)          # a field for the layers above
    s.add("feature", {"id": "beck", "kind": "guide", "path": pts})
    s.warn("too_steep", "cottage_a", "the ground here is 31 degrees", where=(x, y))
    s.done()

This package holds no engine logic. It reads the request, reads the build's files and writes the
answer's files, which any program could do from the contract alone (`worldc/steps.py`). It only
saves you from parsing JSON and reshaping arrays.

Arrays are `[row, column]` with row 0 the south edge: `a[iy, ix]` is the post at
`(x0 + ix * resolution, y0 + iy * resolution)`.
"""
from __future__ import annotations

import json
import os
from pathlib import Path

import numpy as np

CONTRACT = "via-step-0"


class Frame:
    def __init__(self, d: dict) -> None:
        self.x0, self.y0, self.x1, self.y1 = d["extent"]
        self.resolution, self.nx, self.ny, self.seed = d["resolution"], d["nx"], d["ny"], d["seed"]
        self.shape = (self.ny, self.nx)

    def xy(self) -> tuple[np.ndarray, np.ndarray]:
        """World x and y of every post."""
        xs = self.x0 + np.arange(self.nx) * self.resolution
        ys = self.y0 + np.arange(self.ny) * self.resolution
        return np.meshgrid(xs, ys)

    def index(self, x: float, y: float) -> tuple[int, int]:
        """The post nearest a world position, as (row, column), kept inside the grid."""
        ix = int(round((x - self.x0) / self.resolution))
        iy = int(round((y - self.y0) / self.resolution))
        return min(max(iy, 0), self.ny - 1), min(max(ix, 0), self.nx - 1)


class Build:
    """A compiled build, read only."""

    def __init__(self, folder: Path, frame: Frame) -> None:
        self.folder, self.frame = folder, frame
        self.doc = json.loads((folder / "world.json").read_text())
        self.features = {f["id"]: f for f in self.doc["features"]}
        self.explicit = self.doc["explicit"]             # hand-placed instances: id, asset, pos, yaw

    def fields(self) -> list[str]:
        return list(self.doc["fields"])

    def field(self, name: str) -> np.ndarray:
        if name not in self.doc["fields"]:
            raise KeyError(f"the build has no field '{name}'; it has: {', '.join(self.fields())}. "
                           "A field reaches a build only if the world exports it.")
        return np.fromfile(self.folder / self.doc["fields"][name]["file"], dtype=np.float32).reshape(self.frame.shape).copy()

    def instances(self, asset: str | None = None) -> list[dict]:
        """Every placed instance, scattered ones included."""
        rows = []
        for name, chunks in self.doc["instances"].items():
            if asset is None or name == asset:
                rows += [{"asset": name, "pos": r[:3], "yaw": r[3]} for chunk in chunks.values() for r in chunk]
        return rows


class Step:
    def __init__(self, request: dict) -> None:
        if request.get("contract") != CONTRACT:
            raise SystemExit(f"viasdk speaks {CONTRACT}; the compiler sent {request.get('contract')}")
        self.point: str = request["point"]
        self.id: str = request["id"]
        self.params: dict = request["params"]
        self.frame = Frame(request["frame"])
        self.build = Build(Path(request["build"]), self.frame) if request["build"] else None
        self.out = Path(request["out"])
        self._layer: dict = {}
        self._said: list[dict] = []

    # ---- a generate step's answer: a layer
    def write_field(self, name: str, values: np.ndarray, **spec) -> None:
        """Hand the layers above a field. `spec` adds to its declaration, for example unit="m"."""
        values = np.asarray(values, dtype=np.float32)
        if values.shape != self.frame.shape:
            raise ValueError(f"field '{name}' is {values.shape}; the grid is {self.frame.shape} (rows, columns)")
        if not np.isfinite(values).all():
            raise ValueError(f"field '{name}' holds NaN or infinity")
        self.out.mkdir(parents=True, exist_ok=True)
        values.tofile(self.out / f"{name}.f32")
        self._layer.setdefault("field", {})[name] = {"file": f"{name}.f32", **spec}

    def add(self, table: str, row: dict) -> None:
        """Add a row to one of the document's lists: "feature", "instance", "scatter" and so on."""
        self._layer.setdefault(table, []).append(row)

    def set(self, table: str, key: str, value) -> None:
        """Set an entry of one of the document's tables, for example set("params", "treeline", 96.0)."""
        self._layer.setdefault(table, {})[key] = value

    # ---- what any step may say
    def _say(self, severity, code, subject, message, where, hint) -> None:
        row = {"severity": severity, "code": code, "subject": subject, "message": message}
        if where is not None:
            row["where"] = [float(where[0]), float(where[1])]
        if hint:
            row["hint"] = hint
        self._said.append(row)

    def error(self, code, subject, message, where=None, hint="") -> None:
        self._say("error", code, subject, message, where, hint)

    def warn(self, code, subject, message, where=None, hint="") -> None:
        self._say("warning", code, subject, message, where, hint)

    def done(self) -> None:
        self.out.mkdir(parents=True, exist_ok=True)
        if self.point == "generate":
            (self.out / "layer.json").write_text(json.dumps(self._layer, indent=1))
        (self.out / "diagnostics.json").write_text(json.dumps(self._said, indent=1))


def step() -> Step:
    where = os.environ.get("VIA_STEP")
    if not where:
        raise SystemExit("this script is a build step: the compiler runs it and sets VIA_STEP. "
                         "Name it in the world's [[step]] rows and compile the world.")
    return Step(json.loads(Path(where).read_text()))
