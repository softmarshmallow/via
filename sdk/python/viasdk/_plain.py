"""The plain command (protocol.md 2.2 and 8): the request read from ``VIA_STEP``, reads answered
from its ``data``, diagnostics written to ``out/diagnostics.json``.

A plain command makes no engine calls: what it may read is handed to it, by path, and the SDK
computes nothing itself (protocol.md 8). Selection here is by the request's own columns: a field by
name, ``role:<role>`` or ``tag:<tag>``; a feature by ``*``, ``kind:<kind>`` or its name; an instance
by ``*``, its producer's name or ``@<taxon path>``.
"""

import json
import os
import traceback

from ._columns import _NP_DTYPE
from ._errors import NotAWorker
from ._step import Step

SEVERITIES = ("error", "warning", "info")


def _blob_path(objects, digest):
    return os.path.join(objects, digest[:2], digest)


def _column(col, objects):
    """A 4.3 column as a NumPy array (strings as a list), inline or blob."""
    import numpy as np

    dtype = col["dtype"]
    if dtype == "str":
        if "values" in col:
            return list(col["values"])
        with open(_blob_path(objects, col["blob"]), "rb") as f:
            raw = f.read()
        offs = np.fromfile(_blob_path(objects, col["offsets"]), dtype="<i8").tolist()
        return [raw[offs[k]:offs[k + 1]].decode("utf-8") for k in range(len(offs) - 1)]
    shape = tuple(col.get("shape") or ())
    if "values" in col:
        a = np.asarray(col["values"], dtype=_NP_DTYPE[dtype])
        if shape:
            a = a.reshape(shape)
        return a
    path = col.get("path") or _blob_path(objects, col["blob"])
    a = np.fromfile(path, dtype=_NP_DTYPE[dtype])
    if shape:
        a = a.reshape(shape)
    return a


def _take(values, rows):
    if isinstance(values, list):
        return [values[r] for r in rows]
    return values[rows]


class PlainStep(Step):
    """``step`` for a plain command: reads from ``data``, computes raise ``NotAWorker``."""

    def __init__(self, request):
        super().__init__(request)
        self.data = request.get("data") or {}
        self.objects = request.get("objects") or ""
        self.diagnostics = []
        self._explained = set()

    # --- reads -------------------------------------------------------------------------------

    def _field_names(self, selector):
        fields = self.data.get("fields") or {}
        if selector in fields:
            return [selector]
        if selector.startswith("role:"):
            want = selector[len("role:"):]
            return sorted(n for n, f in fields.items() if f.get("role") == want)
        if selector.startswith("tag:"):
            want = selector[len("tag:"):]
            return sorted(n for n, f in fields.items() if want in (f.get("tags") or []))
        if selector == "*":
            return sorted(fields)
        return []

    def fields(self, names, view="full", window=None, halo=0):
        """Read-only mapped arrays (rows from the south), one per selector. A selector matching no
        field raises ``LookupError``; one matching several, ``LookupError`` too."""
        import numpy as np

        out = []
        for sel in names:
            got = self._field_names(sel)
            if len(got) != 1:
                raise LookupError(f"`{sel}` selects {len(got)} fields; a read needs exactly one")
            f = self.data["fields"][got[0]]
            out.append(np.load(f["path"], mmap_mode="r"))
        return out

    def features(self, select, within=None, columns=()):
        """The feature list of 4.3, filtered to ``select`` (``*``, ``kind:<kind>`` or a name)."""
        cols = self.data.get("features") or {}
        dec = {k: _column(v, self.objects) for k, v in cols.items()}
        names = list(dec.get("name", []))
        kinds = list(dec.get("kind", []))
        if select == "*":
            rows = list(range(len(names)))
        elif select.startswith("kind:"):
            want = select[len("kind:"):]
            rows = [r for r, k in enumerate(kinds) if k == want]
        else:
            rows = [r for r, n in enumerate(names) if n == select]
        if not rows and select != "*":
            raise LookupError(f"`{select}` selects no feature")
        return _subset_features(dec, rows)

    def instances(self, select="*", within=None, columns=()):
        """The instance table of 3.3, filtered to ``select`` (``*``, a producer or ``@<taxon>``)."""
        cols = self.data.get("instances") or {}
        dec = {k: _column(v, self.objects) for k, v in cols.items()}
        n = len(dec.get("name", []))
        if select == "*":
            rows = list(range(n))
        elif select.startswith("@"):
            want = select[1:]
            taxa = dec.get("taxon", [""] * n)
            rows = [r for r in range(n) if taxa[r] == want or taxa[r].startswith(want + "/")]
        else:
            by = dec.get("by", [""] * n)
            rows = [r for r in range(n) if by[r] == select]
        return {k: _take(v, rows) for k, v in dec.items()}

    def input(self, path):
        for i in self.data.get("inputs") or []:
            if i["path"] == path:
                with open(i["absolute"], "rb") as f:
                    return f.read()
        raise LookupError(f"`{path}` is not a declared input")

    def call(self, method, **params):
        raise NotAWorker(f"{method} needs a worker (worker = true)")

    def eval(self, formula, x=None, y=None, window=None):
        raise NotAWorker("expr.eval needs a worker (worker = true)")

    # --- diagnostics -------------------------------------------------------------------------

    def _emit(self, severity, code, subject, message, hint, data=None, position=None,
              related=None, explain=None):
        rec = {"severity": severity, "code": str(code), "subject": str(subject),
               "message": str(message), "hint": str(hint)}
        if position is not None:
            rec["position"] = [float(position[0]), float(position[1])]
        if related is not None:
            rec["related"] = list(related)
        if data is not None:
            rec["data"] = dict(data)
        if explain is not None:
            rec["explain"] = str(explain)
        self.diagnostics.append(rec)

    def error(self, code, subject, message, hint, **kw):
        self._emit("error", code, subject, message, hint, **kw)

    def warning(self, code, subject, message, hint, **kw):
        self._emit("warning", code, subject, message, hint, **kw)

    def info(self, code, subject, message, hint, **kw):
        self._emit("info", code, subject, message, hint, **kw)

    # --- what it returns ---------------------------------------------------------------------

    def finish(self):
        """Writes ``diagnostics.json`` and, for a generate step, ``document.toml`` into ``out``."""
        folder = self.request["out"]
        os.makedirs(folder, exist_ok=True)
        if self.when == "generate":
            with open(os.path.join(folder, "document.toml"), "w", encoding="utf-8") as f:
                f.write(self.out.text())
        if self.diagnostics:
            with open(os.path.join(folder, "diagnostics.json"), "w", encoding="utf-8") as f:
                json.dump(self.diagnostics, f, ensure_ascii=False, sort_keys=True)


def _subset_features(dec, rows):
    """Rows of a feature list, with its pieces, rings and coordinates renumbered."""
    import numpy as np

    pieces = [int(v) for v in dec.get("pieces", [0])]
    rings = [int(v) for v in dec.get("rings", [0])]
    starts = [int(v) for v in dec.get("ring_starts", [0])]
    coords = dec.get("coords")
    out = {}
    for k, v in dec.items():
        if k in ("pieces", "rings", "ring_starts", "coords"):
            continue
        out[k] = _take(v, rows)
    new_pieces, new_rings, new_starts, new_coords = [0], [0], [0], []
    for f in rows:
        for p in range(pieces[f], pieces[f + 1]):
            for r in range(rings[p], rings[p + 1]):
                for c in range(starts[r], starts[r + 1]):
                    new_coords.append([int(coords[c][0]), int(coords[c][1])])
                new_starts.append(len(new_coords))
            new_rings.append(len(new_starts) - 1)
        new_pieces.append(len(new_rings) - 1)
    out["pieces"] = np.asarray(new_pieces, dtype="<i8")
    out["rings"] = np.asarray(new_rings, dtype="<i8")
    out["ring_starts"] = np.asarray(new_starts, dtype="<i8")
    out["coords"] = np.asarray(new_coords, dtype="<i4").reshape((len(new_coords), 2))
    return out


def request():
    """The ``step`` of a plain command, from the request ``VIA_STEP`` names (protocol.md 8)."""
    path = os.environ.get("VIA_STEP")
    if not path:
        raise NotAWorker("VIA_STEP is not set: this is not a plain command")
    with open(path, encoding="utf-8") as f:
        return PlainStep(json.load(f))


def run(script):
    """``python -m viasdk <script>`` as a plain command: import the script, call the function
    named after ``when``, write what ``step`` holds. Returns the exit status."""
    import importlib.util
    import sys

    try:
        step = request()
        script = os.path.abspath(script)
        folder = os.path.dirname(script)
        if folder not in sys.path:
            sys.path.insert(0, folder)
        name = os.path.splitext(os.path.basename(script))[0]
        spec = importlib.util.spec_from_file_location(name, script)
        module = importlib.util.module_from_spec(spec)
        sys.modules[name] = module
        spec.loader.exec_module(module)
        fn = getattr(module, step.when, None)
        if not callable(fn):
            print(f"viasdk: {script} has no function `{step.when}(step)`", file=sys.stderr)
            return 1
        fn(step)
        step.finish()
    except Exception:  # noqa: BLE001 - every failure is the log's, and protocol.failed
        traceback.print_exc()
        return 1
    return 0
