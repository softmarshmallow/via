"""Columns in messages (protocol.md 4.3): inline JSON up to 4,096 values, blobs beyond.

Every engine call is a batch of columns (R23), so this is the whole payload of a call.
"""

INLINE_MAX = 4096

_NP_DTYPE = {"u8": "<u1", "bool": "<u1", "i32": "<i4", "i64": "<i8", "f32": "<f4", "f64": "<f8"}


def to_wire(values, dtype):
    """A column for a message. Accepts a NumPy array or a sequence of Python numbers."""
    try:
        import numpy as np
    except ImportError:  # pragma: no cover - numpy is optional
        np = None
    if np is not None and isinstance(values, np.ndarray):
        if values.ndim != 1:
            raise ValueError("the probe sends one-dimensional columns only")
        if dtype in ("f32", "f64"):
            if not np.isfinite(values).all():
                raise ValueError("a non-finite value in a column (R26)")
            values = values + 0.0  # -0.0 as +0.0 (protocol.md 4.2)
        values = values.tolist()
    else:
        values = list(values)
    if len(values) > INLINE_MAX:
        # A blob column needs its BLAKE3 name, which the standard library cannot compute; how a
        # pure-Python SDK names a blob is protocol.md's open question (build.md 3.3).
        raise NotImplementedError(f"columns over {INLINE_MAX} values travel as blobs (M2)")
    return {"dtype": dtype, "values": values}


def from_wire(col):
    """A column from a message: a NumPy array (mapped read-only when it is a blob), or a list when
    NumPy is not installed and the column is inline."""
    dtype = col["dtype"]
    if "values" in col:
        try:
            import numpy as np
        except ImportError:  # pragma: no cover - numpy is optional
            return list(col["values"])
        if dtype == "str":
            return list(col["values"])
        return np.asarray(col["values"], dtype=_NP_DTYPE[dtype])
    import numpy as np

    shape = tuple(col.get("shape") or ())
    n = 1
    for d in shape:
        n *= d
    if n == 0:
        return np.zeros(shape, dtype=_NP_DTYPE[dtype])
    return np.memmap(col["path"], dtype=_NP_DTYPE[dtype], mode="r", shape=shape)
