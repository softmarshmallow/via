"""The ``step`` object a step function receives (protocol.md section 8)."""

import json
import os

from ._columns import from_wire, to_wire
from ._errors import Cancelled, NotAWorker


class Out:
    """The document builder: what the step returns. It writes text and field files and holds
    nothing else (R7). The probe needs fields only; features, instances and rows arrive in M2."""

    def __init__(self, folder):
        self.folder = folder
        self._tables = []

    def field(self, name, array, tags=()):
        """A field of the step's own (R32), written as ``<name>.npy`` in ``out``."""
        from ._npy import write_field

        file = f"{name}.npy"
        write_field(os.path.join(self.folder, file), array)
        lines = [f"[field.{name}]", f"file = {_toml_str(file)}"]
        if tags:
            lines.append("tags = [" + ", ".join(_toml_str(t) for t in sorted(tags)) + "]")
        self._tables.append("\n".join(lines) + "\n")

    def text(self):
        """The document as TOML text; empty when nothing was returned (protocol.md 5)."""
        return "\n".join(self._tables)


def _toml_str(s):
    # A TOML basic string; JSON's escapes are a subset TOML accepts for the text we write.
    return json.dumps(str(s), ensure_ascii=False)


class Step:
    """One call of a step: the request, the engine's reads and computes, and ``out``."""

    def __init__(self, request, conn=None, call_id=None):
        self.request = request
        self.id = request["step"]
        self.when = request["when"]
        self.params = request.get("params") or {}
        self.grid = request.get("grid")
        self.seed = request.get("seed")
        self.for_each_item = request.get("for_each_item")
        self.out = Out(request["out"])
        self._conn = conn
        self._call = call_id

    def call(self, method, **params):
        """One engine call, with ``call`` set to this step.call's id (protocol.md 3.3)."""
        if self._conn is None:
            raise NotAWorker(f"{method} needs a worker (worker = true)")
        if self._call in self._conn.cancelled:
            raise Cancelled()
        params["call"] = self._call
        try:
            return self._conn.request(method, params)
        except Exception as e:
            if self._call in self._conn.cancelled:
                raise Cancelled() from e
            raise

    def fields(self, names, view="full", window=None, halo=0):
        """build.field: one read-only mapped array per name, rows from the south."""
        import numpy as np

        params = {"fields": list(names), "view": view, "halo": halo}
        if window is not None:
            params["window"] = window
        got = self.call("build.field", **params)
        out = []
        for n in names:
            f = got[n]
            a = np.load(f["path"], mmap_mode="r")
            out.append(a)
        return out

    def eval(self, formula, x=None, y=None, window=None):
        """expr.eval: one batch, one column back (f64 or bool)."""
        params = {"formula": formula}
        if window is not None:
            params["window"] = window
        else:
            params["x"] = to_wire(x, "i32")
            params["y"] = to_wire(y, "i32")
        return from_wire(self.call("expr.eval", **params))
