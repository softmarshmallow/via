"""Columns in messages (protocol.md 4.3): inline JSON up to 4,096 values, blobs beyond.

Every engine call is a batch of columns (R23), so this is the whole payload of a call. A blob is
written to ``<objects>/tmp/`` and moved to ``<objects>/ab/<digest>`` by a move that never replaces
(4.1); its name is its BLAKE3 digest (``_blake3``).
"""

import json
import os
import uuid

INLINE_MAX = 4096

_NP_DTYPE = {"u8": "<u1", "bool": "<u1", "i32": "<i4", "i64": "<i8", "f32": "<f4", "f64": "<f8"}


def blob_path(objects, digest):
    return os.path.join(objects, digest[:2], digest)


def put_blob(objects, data):
    """Writes bytes as a blob in the store and returns its name. A blob already there is kept."""
    from ._blake3 import digest as b3

    name = b3(data)
    dest = blob_path(objects, name)
    if os.path.exists(dest):
        return name
    tmp_dir = os.path.join(objects, "tmp")
    os.makedirs(tmp_dir, exist_ok=True)
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    tmp = os.path.join(tmp_dir, f"viasdk-{os.getpid()}-{uuid.uuid4().hex}")
    with open(tmp, "wb") as f:
        f.write(data)
    try:
        os.link(tmp, dest)  # never replaces: fails if the name exists (4.1)
    except FileExistsError:
        pass
    except OSError:
        # A system without hard links: a rename that refuses to replace is the next best.
        if not os.path.exists(dest):
            os.rename(tmp, dest)
            return name
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)
    return name


def to_wire(values, dtype, objects=None):
    """A column for a message, from a NumPy array or a sequence. ``dtype`` is one of 4.3's. A
    two-dimensional array keeps its shape. More than 4,096 values go as a blob, which needs the
    store's ``objects`` folder."""
    if dtype == "str":
        values = [str(v) for v in values]
        if len(values) <= INLINE_MAX:
            return {"dtype": "str", "values": values}
        if objects is None:
            raise ValueError("a column over 4,096 values needs the store")
        raw = b""
        offs = [0]
        parts = []
        for v in values:
            b = v.encode("utf-8")
            parts.append(b)
            offs.append(offs[-1] + len(b))
        raw = b"".join(parts)
        import numpy as np

        o = np.asarray(offs, dtype="<i8").tobytes()
        return {"dtype": "str", "blob": put_blob(objects, raw), "offsets": put_blob(objects, o)}
    import numpy as np

    a = np.asarray(values)
    if dtype in ("f32", "f64"):
        a = a.astype(np.float64)
        if not np.isfinite(a).all():
            raise ValueError("a non-finite value in a column (R26)")
        a = a + 0.0  # -0.0 as +0.0 (protocol.md 4.2)
    elif dtype == "bool":
        a = a.astype(bool).astype(np.uint8)
    a = np.ascontiguousarray(a, dtype=_NP_DTYPE[dtype])
    shape = list(a.shape)
    if a.size <= INLINE_MAX:
        flat = a.reshape(-1).tolist()
        if dtype == "bool":
            flat = [bool(x) for x in flat]
        col = {"dtype": dtype, "values": flat}
        if len(shape) > 1:
            col["shape"] = shape
        return col
    if objects is None:
        raise ValueError("a column over 4,096 values needs the store")
    return {"dtype": dtype, "shape": shape, "blob": put_blob(objects, a.tobytes())}


def from_wire(col, objects=None):
    """A column from a message: a NumPy array (mapped read-only when it is a blob), or a list of
    strings."""
    import numpy as np

    dtype = col["dtype"]
    shape = tuple(col.get("shape") or ())
    if dtype == "str":
        if "values" in col:
            return list(col["values"])
        with open(blob_path(objects, col["blob"]), "rb") as f:
            raw = f.read()
        offs = np.fromfile(blob_path(objects, col["offsets"]), dtype="<i8").tolist()
        return [raw[offs[k]:offs[k + 1]].decode("utf-8") for k in range(len(offs) - 1)]
    if "values" in col:
        a = np.asarray(col["values"], dtype=_NP_DTYPE[dtype])
        if shape:
            a = a.reshape(shape)
        return a
    n = 1
    for d in shape:
        n *= d
    if n == 0:
        return np.zeros(shape, dtype=_NP_DTYPE[dtype])
    path = col.get("path") or blob_path(objects, col["blob"])
    return np.memmap(path, dtype=_NP_DTYPE[dtype], mode="r", shape=shape)


def table(cols, objects=None):
    """A table or feature list from a message: each column decoded."""
    return {k: from_wire(v, objects) for k, v in (cols or {}).items()}


def canonical(value):
    """Canonical JSON text (documents.md 8): members sorted, no spaces."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)
