"""The ``step`` object a step function receives (protocol.md section 8).

Every engine call is a batch (R23): a method takes columns and gives columns, never one point at a
time. Coordinates are integer millimetres in the world frame (R24); ``FeatureList.metres`` gives
metres when a step wants them.
"""

import json
import os

from ._columns import canonical, from_wire, table, to_wire
from ._errors import Cancelled, NotAWorker

SEVERITIES = ("error", "warning", "info")


def _toml_str(s):
    # A TOML basic string; JSON's escapes are a subset TOML accepts for the text we write.
    return json.dumps(str(s), ensure_ascii=False)


def metres(mm):
    """Exact decimal metres from integer millimetres, as the text writes a length (R24)."""
    mm = int(mm)
    sign = "-" if mm < 0 else ""
    a = abs(mm)
    whole, frac = divmod(a, 1000)
    if frac == 0:
        return f"{sign}{whole}"
    return f"{sign}{whole}.{frac:03d}".rstrip("0")


def _toml_value(v):
    """A TOML value for an attribute or a parameter: strings, bools, ints, floats, lists, tables."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        if v != v or v in (float("inf"), float("-inf")):
            raise ValueError("no non-finite value enters a build (R26)")
        return repr(v + 0.0)
    if isinstance(v, str):
        return _toml_str(v)
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(_toml_value(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{_key(k)} = {_toml_value(x)}" for k, x in v.items()) + " }"
    try:
        import numpy as np

        if isinstance(v, np.generic):
            return _toml_value(v.item())
    except ImportError:  # pragma: no cover
        pass
    raise TypeError(f"cannot write {type(v).__name__} in a document")


def _key(k):
    k = str(k)
    if k and all(c.isalnum() or c in "_-" for c in k):
        return k
    return _toml_str(k)


def _point(p):
    """``[x, y]`` in metres, exactly, from millimetres."""
    return f"[{metres(p[0])}, {metres(p[1])}]"


class Out:
    """The document builder: what a generate step returns (protocol.md 5). It writes text and field
    files and holds nothing else (R7). Coordinates are integer millimetres; the document gets exact
    metres."""

    def __init__(self, folder):
        self.folder = folder
        self._tables = []

    def field(self, name, array, tags=(), unit=None, role=None, outside=0):
        """A field of the step's own (R32), written as ``<name>.npy`` in ``out``: a new name, read
        in this document as ``f('<name>')`` and from above by a tag it sets."""
        from ._npy import write_field

        file = f"{name}.npy"
        write_field(os.path.join(self.folder, file), array)
        lines = [f"[field.{_key(name)}]", f"file = {_toml_str(file)}"]
        if role is not None:
            lines.append(f"role = {_toml_str(role)}")
        if tags:
            lines.append("tags = [" + ", ".join(_toml_str(t) for t in sorted(tags)) + "]")
        if unit is not None:
            lines.append(f"unit = {_toml_str(unit)}")
        lines.append(f"outside = {_toml_value(outside)}")
        self._tables.append("\n".join(lines) + "\n")

    def row(self, field, id, value=None, mask=None, paint=None, order=None):
        """A ``then`` row on a field below the step (fields.md 7): ``value`` and ``mask`` are
        formulas or numbers."""
        lines = [f"[field.{_key(field)}.then.{_key(id)}]"]
        for k, v in (("value", value), ("mask", mask), ("paint", paint), ("order", order)):
            if v is not None:
                lines.append(f"{k} = {_toml_value(v)}")
        self._tables.append("\n".join(lines) + "\n")

    def feature(self, key, kind, at=None, path=None, region=None, authority=None, **attrs):
        """A feature: a point ``at``, a ``path`` or a ``region`` ring, in millimetres."""
        lines = [f"[feature.{_key(key)}]", f"kind = {_toml_str(kind)}"]
        if authority is not None:
            lines.append(f"authority = {_toml_str(authority)}")
        if at is not None:
            lines.append(f"at = {_point(at)}")
        if path is not None:
            lines.append("path = [" + ", ".join(_point(p) for p in path) + "]")
        if region is not None:
            lines.append("region = [" + ", ".join(_point(p) for p in region) + "]")
        for k in sorted(attrs):
            lines.append(f"{_key(k)} = {_toml_value(attrs[k])}")
        self._tables.append("\n".join(lines) + "\n")

    def instance(self, key, asset, at, face=None, **keys):
        """An instance of ``asset`` at ``at`` (millimetres); ``face`` in degrees."""
        lines = [f"[instance.{_key(key)}]", f"asset = {_toml_str(asset)}", f"at = {_point(at)}"]
        if face is not None:
            lines.append(f"face = {_toml_value(face)}")
        for k in sorted(keys):
            lines.append(f"{_key(k)} = {_toml_value(keys[k])}")
        self._tables.append("\n".join(lines) + "\n")

    def text(self):
        """The document as TOML text; empty when nothing was returned (protocol.md 5)."""
        return "\n".join(self._tables)


class FeatureList(dict):
    """A feature list (protocol.md 4.3): columns by name, with helpers to walk the geometry."""

    def __len__(self):
        return len(self.get("geometry", []))

    @property
    def names(self):
        return list(self.get("name", []))

    def rings(self, f):
        """The rings of feature ``f`` by piece: a list of pieces, each a list of rings, each an
        ``(n, 2)`` array of millimetres."""
        pieces, rings, starts, coords = self["pieces"], self["rings"], self["ring_starts"], self["coords"]
        out = []
        for p in range(int(pieces[f]), int(pieces[f + 1])):
            out.append([coords[int(starts[r]):int(starts[r + 1])] for r in range(int(rings[p]), int(rings[p + 1]))])
        return out

    def points(self, f):
        """Every vertex of feature ``f``, in order, as an ``(n, 2)`` array of millimetres."""
        import numpy as np

        rs = [r for piece in self.rings(f) for r in piece]
        return np.concatenate(rs) if rs else np.zeros((0, 2), dtype="<i4")

    def metres(self):
        """Every vertex in metres, as doubles."""
        import numpy as np

        return np.asarray(self["coords"], dtype=np.float64) / 1000.0


def feature_list(paths, objects=None):
    """A feature list of paths, from lists of ``(x, y)`` millimetres, for ``route``'s waypoints."""
    import numpy as np

    coords, pieces, rings, starts = [], [0], [0], [0]
    for p in paths:
        coords.extend([int(x), int(y)] for x, y in p)
        starts.append(len(coords))
        rings.append(len(starts) - 1)
        pieces.append(len(rings) - 1)
    c = np.asarray(coords, dtype="<i4").reshape((len(coords), 2))
    return {
        "coords": to_wire(c, "i32", objects),
        "geometry": to_wire([1] * len(paths), "u8", objects),
        "pieces": to_wire(pieces, "i64", objects),
        "rings": to_wire(rings, "i64", objects),
        "ring_starts": to_wire(starts, "i64", objects),
    }


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
        self.checks = request.get("checks")
        self.objects = request.get("objects") or ""
        self.out = Out(request["out"])
        self._conn = conn
        self._call = call_id
        self._diags = []

    # --- the wire ------------------------------------------------------------------------------

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

    def _col(self, values, dtype):
        return to_wire(values, dtype, self.objects)

    # --- traced reads --------------------------------------------------------------------------

    def fields(self, names, view="full", window=None, halo=0):
        """build.field: one read-only mapped array per name or selector, rows from the south. Each
        array has ``window`` (its ``i0, j0, i1, j1``) and, for weights, ``materials``."""
        import numpy as np

        params = {"fields": list(names), "view": view, "halo": halo}
        if window is not None:
            params["window"] = list(window)
        got = self.call("build.field", **params)
        out = []
        for n in names:
            f = got[n]
            a = np.load(f["path"], mmap_mode="r")
            a.window = (f["i0"], f["j0"], f["i1"], f["j1"])
            a.name = f.get("name", n)
            a.materials = f.get("materials")
            out.append(a)
        return out

    def features(self, select="*", within=None, columns=()):
        """build.features: a FeatureList (protocol.md 4.3), coordinates in millimetres."""
        params = {"select": select if isinstance(select, str) else list(select),
                  "columns": columns if isinstance(columns, str) else list(columns)}
        if within is not None:
            params["within"] = within if isinstance(within, str) else [list(map(int, p)) for p in within]
        return FeatureList(table(self.call("build.features", **params), self.objects))

    def instances(self, select="*", within=None, columns=()):
        """build.instances: a table of instances."""
        params = {"select": select}
        if columns:
            params["columns"] = list(columns)
        if within is not None:
            params["within"] = within if isinstance(within, str) else [list(map(int, p)) for p in within]
        return table(self.call("build.instances", **params), self.objects)

    def input(self, path):
        """input.open: a declared input's bytes."""
        got = self.call("input.open", path=path)
        with open(got["path"], "rb") as f:
            return f.read()

    def pack(self, select="*", columns=()):
        """pack.query: a table by asset id."""
        params = {"select": select if isinstance(select, str) else list(select),
                  "columns": columns if isinstance(columns, str) else list(columns)}
        return table(self.call("pack.query", **params), self.objects)

    # --- computes ------------------------------------------------------------------------------

    def eval(self, formula, x=None, y=None, window=None, view="full"):
        """expr.eval: one batch, one column back (f64), with the engine's maths (R26)."""
        params = {"formula": formula, "view": view}
        if window is not None:
            params["window"] = list(window)
        else:
            params["x"] = self._col(x, "i32")
            params["y"] = self._col(y, "i32")
        return from_wire(self.call("expr.eval", **params), self.objects)

    def derive_where(self, formulas, holds=None, view="full"):
        """derive.where: one region per formula, each kept to the piece holding its point when
        ``holds`` gives one per formula; a FeatureList with a bool column ``dry``."""
        params = {"formulas": self._col(list(formulas), "str"), "view": view}
        if holds is not None:
            import numpy as np

            params["holds"] = self._col(np.asarray(holds, dtype="<i4").reshape((-1, 2)), "i32")
        return FeatureList(table(self.call("derive.where", **params), self.objects))

    def route(self, cost, waypoints, view="full"):
        """derive.route: one course per list of waypoints (millimetres), over the least cost per
        metre ``cost``; a FeatureList with an f64 column ``cost``."""
        params = {"cost": cost, "waypoints": feature_list(waypoints, self.objects), "view": view}
        return FeatureList(table(self.call("derive.route", **params), self.objects))

    def descend(self, field, x, y, view="full"):
        """derive.descend: a flow line from each point; a FeatureList with a str column ``end``."""
        params = {"field": field, "x": self._col(x, "i32"), "y": self._col(y, "i32"), "view": view}
        return FeatureList(table(self.call("derive.descend", **params), self.objects))

    def spill(self, field, x, y, radius=None, view="full"):
        """derive.spill: the level each point's hollow holds to, and whether it is dry."""
        params = {"field": field, "x": self._col(x, "i32"), "y": self._col(y, "i32"), "view": view}
        if radius is not None:
            params["radius"] = self._col(radius, "i32")
        return table(self.call("derive.spill", **params), self.objects)

    # --- says ----------------------------------------------------------------------------------

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
        self._diags.append(rec)

    def error(self, code, subject, message, hint, **kw):
        self._emit("error", code, subject, message, hint, **kw)

    def warning(self, code, subject, message, hint, **kw):
        self._emit("warning", code, subject, message, hint, **kw)

    def info(self, code, subject, message, hint, **kw):
        self._emit("info", code, subject, message, hint, **kw)

    def flush(self):
        """Sends what the step said as diag.emit batches (R23): one for the records with a
        position and one for those without, since a column has a value in every row."""
        if not self._diags or self._conn is None:
            return
        rows = self._diags
        self._diags = []
        for group in ([r for r in rows if "position" in r], [r for r in rows if "position" not in r]):
            if not group:
                continue
            params = {k: self._col([r[k] for r in group], "str")
                      for k in ("severity", "code", "subject", "message", "hint")}
            params["related"] = self._col([canonical(r["related"]) if "related" in r else "" for r in group], "str")
            params["data"] = self._col([canonical(r["data"]) if "data" in r else "" for r in group], "str")
            params["explain"] = self._col([r.get("explain", "") for r in group], "str")
            if "position" in group[0]:
                params["position"] = {"dtype": "f64", "shape": [len(group), 2],
                                      "values": [v for r in group for v in r["position"]]}
            self.call("diag.emit", **params)
