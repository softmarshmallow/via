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

    def _view(self, view):
        # A plain command is handed one view (protocol.md 2.2): a generate step's stratum in the
        # full view, a lint's or export's finished build, where nothing stands above it and the
        # head view is the same. Any other is not in ``data``: refused, never answered with
        # another.
        if view == "full" or (view == "head" and self.when != "generate"):
            return
        raise NotAWorker(
            f"view={view!r}: a plain {self.when} step is handed only the full view; "
            "read the head view from a worker (worker = true)"
        )

    def _window(self, window, halo):
        g = self.grid or {}
        nx, ny = int(g.get("nx", 0)), int(g.get("ny", 0))
        if window is None:
            w = [0, 0, nx, ny]
        elif isinstance(window, dict):
            w = [int(window[k]) for k in ("i0", "j0", "i1", "j1")]
        else:
            w = [int(v) for v in window]
            if len(w) != 4:
                raise ValueError("window: give i0, j0, i1, j1")
        h = max(int(halo or 0), 0)
        i0, j0 = max(w[0] - h, 0), max(w[1] - h, 0)
        i1, j1 = min(w[2] + h, nx), min(w[3] + h, ny)
        if i1 <= i0 or j1 <= j0:
            raise ValueError("window: empty, or outside the grid")
        return i0, j0, i1, j1

    def fields(self, names, view="full", window=None, halo=0):
        """Read-only mapped arrays (rows from the south), one per selector, each cut to ``window``
        grown by ``halo`` and clipped to the grid, as a worker's read is (protocol.md 3.3). A
        selector matching no field, or several, raises ``LookupError``; a view the request does
        not hold raises ``NotAWorker``."""
        import numpy as np

        self._view(view)
        out = []
        for sel in names:
            got = self._field_names(sel)
            if len(got) != 1:
                raise LookupError(f"`{sel}` selects {len(got)} fields; a read needs exactly one")
            f = self.data["fields"][got[0]]
            whole = np.load(f["path"], mmap_mode="r")
            if self.grid:
                i0, j0, i1, j1 = self._window(window, halo)
            elif window is None:
                i0, j0, i1, j1 = 0, 0, whole.shape[-1], whole.shape[-2]
            else:
                raise ValueError("window: the request has no grid to cut it from")
            a = whole[..., j0:j1, i0:i1]
            a.window = (i0, j0, i1, j1)
            a.name = got[0]
            a.materials = f.get("materials")
            out.append(a)
        return out

    def _region(self, within, dec):
        """``within`` as rings of integer millimetres: a region feature's name, or a ring."""
        if within is None:
            return None
        if isinstance(within, str):
            names = list(dec.get("name", []))
            if within not in names:
                raise LookupError(f"no feature is named `{within}`")
            f = names.index(within)
            geom = list(dec.get("geometry", []))
            if geom and int(geom[f]) != 2:
                raise ValueError(f"`within`: `{within}` is not a region")
            pieces = [int(v) for v in dec.get("pieces", [0])]
            rings = [int(v) for v in dec.get("rings", [0])]
            starts = [int(v) for v in dec.get("ring_starts", [0])]
            coords = dec.get("coords")
            out = []
            for p in range(pieces[f], pieces[f + 1]):
                for r in range(rings[p], rings[p + 1]):
                    out.append([(int(coords[c][0]), int(coords[c][1]))
                                for c in range(starts[r], starts[r + 1])])
            return out
        return [[(int(p[0]), int(p[1])) for p in within]]

    def features(self, select, within=None, columns=()):
        """The feature list of 4.3, filtered to ``select`` (``*``, ``kind:<kind>`` or a name) and,
        with ``within``, to features with a vertex in that region (its edge included)."""
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
        region = self._region(within, dec)
        if region is not None:
            pieces = [int(v) for v in dec.get("pieces", [0])]
            rings = [int(v) for v in dec.get("rings", [0])]
            starts = [int(v) for v in dec.get("ring_starts", [0])]
            coords = dec.get("coords")

            def vertices(f):
                for p in range(pieces[f], pieces[f + 1]):
                    for r in range(rings[p], rings[p + 1]):
                        for c in range(starts[r], starts[r + 1]):
                            yield int(coords[c][0]), int(coords[c][1])

            rows = [f for f in rows if any(_inside(region, v) for v in vertices(f))]
        return _subset_features(dec, rows)

    def instances(self, select="*", within=None, columns=()):
        """The instance table of 3.3, filtered to ``select`` (``*``, a producer or ``@<taxon>``)
        and, with ``within``, to instances whose origin lies in that region (its edge included)."""
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
        if within is not None:
            fcols = self.data.get("features") or {}
            fdec = {k: _column(v, self.objects) for k, v in fcols.items()}
            region = self._region(within, fdec)
            xs, ys = dec.get("x", []), dec.get("y", [])
            rows = [r for r in rows if _inside(region, (int(xs[r]), int(ys[r])))]
        return {k: _take(v, rows) for k, v in dec.items()}

    def input(self, path):
        for i in self.data.get("inputs") or []:
            if i["path"] == path:
                with open(i["absolute"], "rb") as f:
                    return f.read()
        raise LookupError(f"`{path}` is not a declared input")

    def pack(self, select="*", columns=()):
        """The pack's table (``data.pack``, protocol.md 2.2), filtered to ``select``."""
        cols = self.data.get("pack") or {}
        dec = {k: _column(v, self.objects) for k, v in cols.items()}
        ids = list(dec.get("id", []))
        if select == "*":
            rows = list(range(len(ids)))
        elif isinstance(select, str) and select.startswith("@"):
            want = select[1:]
            taxa = dec.get("taxon", [""] * len(ids))
            rows = [r for r in range(len(ids)) if taxa[r] == want or taxa[r].startswith(want + "/")]
        else:
            want = {select} if isinstance(select, str) else set(select)
            rows = [r for r in range(len(ids)) if ids[r] in want]
        keep = None if columns in ("*", ()) else {"id", "source", "taxon", "tags", *columns}
        return {k: _take(v, rows) for k, v in dec.items() if keep is None or k in keep}

    def call(self, method, **params):
        raise NotAWorker(f"{method} needs a worker (worker = true)")

    def eval(self, formula, x=None, y=None, window=None, view="full"):
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
            # A script that wrote document.toml itself, and returned nothing through step.out,
            # keeps its own file (protocol.md 5).
            doc = os.path.join(folder, "document.toml")
            if self.out.text() or not os.path.exists(doc):
                with open(doc, "w", encoding="utf-8") as f:
                    f.write(self.out.text())
        if self.diagnostics:
            with open(os.path.join(folder, "diagnostics.json"), "w", encoding="utf-8") as f:
                json.dump(self.diagnostics, f, ensure_ascii=False, sort_keys=True)


def _in_ring(ring, p):
    """Whether ``p`` lies in ``ring``: ``None`` on its edge, exactly, in integers (geometry.md)."""
    n = len(ring)
    inside = False
    for k in range(n):
        a, b = ring[k], ring[(k + 1) % n]
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        if (cross == 0 and min(a[0], b[0]) <= p[0] <= max(a[0], b[0])
                and min(a[1], b[1]) <= p[1] <= max(a[1], b[1])):
            return None
        if (a[1] > p[1]) != (b[1] > p[1]):
            lhs = (p[0] - a[0]) * (b[1] - a[1])
            rhs = (b[0] - a[0]) * (p[1] - a[1])
            if (lhs < rhs) if b[1] > a[1] else (lhs > rhs):
                inside = not inside
    return inside


def _inside(rings, p):
    """Whether ``p`` lies in these rings read as one region by even-odd, edges included."""
    n = 0
    for r in rings:
        got = _in_ring(r, p)
        if got is None:
            return True
        n += int(got)
    return n % 2 == 1


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
