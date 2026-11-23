"""A kit's pieces, a feature's attributes and the ground under points: three reads every library
under lib/ wrote again for itself (docs/design/kit-review.md, section 4: six, five and six copies;
the fix plan's F4, docs/plan-fixes.md). Plumbing only: each is one engine read, decoded, and
computes nothing a library decides (protocol.md 3.3). They take the step, so any object with the
step's reads (a test's fake step) serves."""

import json

from ._errors import RpcError

# The columns of a feature list that are its geometry, not its attributes (protocol.md 4.3).
GEOMETRY = ("name", "kind", "authority", "layer", "rank", "class", "geometry", "coords", "pieces",
            "rings", "ring_starts")


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _json(v):
    """A text column's value: JSON decoded where it is JSON, else the text; None for empty text."""
    if isinstance(v, str):
        try:
            return json.loads(v) if v else None
        except ValueError:
            return v
    return v


def kit(step, taxon, columns=("bounds", "params", "shared")):
    """`pack.query` of the pieces filed at `taxon` or below it, in its order: one dict per piece,
    `id`, `taxon` (the one asked, not the one the piece is filed at), `tags` (a list), and each
    column asked (one name, or several): `bounds` six whole
    millimetres (None where the pack gives none), `params` and `shared` tables ({} where empty),
    `footprint` a list ([] where empty), `sink` metres (0 where absent), any other as given. A
    kit's `shared` is the same table on each of its pieces (spec/assets.md 2.5); which one a library
    takes, and what two tables mean, is the library's."""
    columns = (columns,) if isinstance(columns, str) else tuple(columns)
    t = step.pack(select="@" + taxon, columns=list(columns))
    rows = []
    for r, i in enumerate(_list(t.get("id", []))):
        tags = _json(_list(t["tags"])[r]) if "tags" in t else []
        row = {"id": str(i), "taxon": taxon, "tags": tags if isinstance(tags, list) else [tags]}
        for c in columns:
            v = _list(t[c])[r] if c in t else None
            if c == "bounds":
                row[c] = [int(x) for x in _list(v)] if v is not None else None
            elif c in ("params", "shared"):
                row[c] = (_json(v) or {}) if v is not None else {}
            elif c == "footprint":
                row[c] = (_json(v) or []) if v is not None else []
            elif c == "sink":
                row[c] = float(v) if v is not None else 0.0
            else:
                row[c] = _json(v)
        rows.append(row)
    return rows


def attrs(fl, f, names=None):
    """The attributes of feature `f` of a feature list, by name: each column `names` lists (every
    column but the geometry's where None) that the feature has, past its `<name>?` null column
    (protocol.md 4.3). An attribute its `?` column marks null is absent. Text is decoded as JSON
    where it is JSON, as every library read it before (a list or a table travels as JSON text):
    so a text attribute "12" or "true" comes back a number or a bool, and an empty text None."""
    if names is None:
        names = [k for k in fl if not k.endswith("?") and k not in GEOMETRY]
    out = {}
    for a in names:
        if a in fl and (f"{a}?" not in fl or _list(fl[f"{a}?"])[f]):
            out[a] = _json(_list(fl[a])[f])
    return out


def ground(step, x, y, view="full"):
    """The terrain at each point (`x`, `y` millimetres), metres, as floats: the world's surface
    field (`role:surface`) read by `expr.eval` in the step's view. None where the world has no
    surface field."""
    kw = {} if view == "full" else {"view": view}
    try:
        # In the view asked, the field too: a step that returns rows on the surface must not read
        # its full view (authority.md 4.4, `authority.step_view`; F4's review).
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1], **kw)
    except (LookupError, RpcError):
        return None
    x, y = list(x), list(y)
    if not x:
        return []
    return [float(v) for v in _list(step.eval(f"= f('{surface.name}')", x=x, y=y, **kw))]
