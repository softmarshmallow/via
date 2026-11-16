"""The ways library's parapet step: a run of posts along each edge of a crossing's deck.

A generate step, `for_each = "kind:crossing"` (kinds.toml shows the table a world restates). It
reads the crossing's path and `width` (`build.features`) and the parapet piece's `bounds`
(`pack.query`), and returns two runs (rules.md 5), one along each edge of the deck:

    [run.<crossing>_parapet_l]
    asset   = "<asset>"
    on      = { ref = "<crossing>", span = [s, 1 - s], offset = <o> }
    spacing = <spacing>
    ends    = "both"

with `o = width / 2 - w / 2` to the left and `-o` to the right, `w` the piece's width across the
deck (its `bounds` x extent: a member faces along the path, so its x is across it). So every post
stands inside the deck's bound (surfaces.md 1.2: the edge counts as inside), its outer face on the
edge. The span is `[s, 1 - s]` with `s = (d / 2) / L`, `d` the piece's depth along the deck (its
`bounds` y extent) and `L` the deck's length along its path: the posts at the ends stand with
their end faces on the deck's ends, not half a post past them (TODO "From M5 slice 2"). A run `on` the crossing stands on the crossing's one surface, the deck (surfaces.md 4.1), and
moves with it; a member is never at the terrain under the deck (TODO "From M5 slice 1": until the
core grounds a run on its surface, its members stand on the terrain).

`params`: `asset` (an asset id, or `@<taxon path>`: the first id under it bytewise; default
`@prop/fence`), `spacing` (metres between posts, default 2). Codes are `step.<id>.<code>`:
`params` (error), `no_piece` (warning: no asset to place, nothing returned), `narrow` (warning:
the piece leaves no deck between the rows), `short` (warning: the end insets leave no length
along the deck), `laid` (info).

Ours, written for this library (surfaces.md 5: "a deck's parapet as a run"). Library code imports
only viasdk and the standard library (R19b).
"""

import json

DEFAULTS = {"asset": "@prop/fence", "spacing": 2.0}

EXPLAIN = {
    "params": "A param of the ways library's parapet step is outside what it takes: `asset` an asset "
    "id or `@<taxon path>`, `spacing` a number of metres above 0.",
    "no_piece": "The parapet step found no asset to place: the pack holds no asset by that id or under "
    "that taxon. Nothing is placed along the deck.",
    "narrow": "The parapet piece is as wide as the deck or wider: two rows of it would not leave the "
    "deck open. Nothing is placed.",
    "short": "The parapet piece is as deep along the deck as the deck is long, or deeper: inset by half "
    "its depth at each end, the run has no length left. Nothing is placed.",
    "laid": "What the parapet step returned for one crossing: two runs of posts, one along each edge "
    "of its deck.",
}


class ParamError(ValueError):
    pass


def params_of(raw):
    p = dict(DEFAULTS)
    for k, v in (raw or {}).items():
        if k not in DEFAULTS:
            raise ParamError(f"`{k}` is no param of the parapet step")
        p[k] = v
    if not isinstance(p["asset"], str) or not p["asset"]:
        raise ParamError("`asset` must be an asset id or `@<taxon path>`")
    try:
        p["spacing"] = float(p["spacing"])
    except (TypeError, ValueError):
        raise ParamError("`spacing` must be a number")
    if not p["spacing"] > 0:
        raise ParamError("`spacing` must be above 0")
    return p


def key_of(name):
    """A key from the crossing's name: characters an id may not hold become `_` (identity.md 2.1)."""
    return "".join(c if c.isalnum() or c in "_-" else "_" for c in str(name))


def runs(name, width, piece_id, piece_w_mm, spacing, piece_d_mm=0, length_mm=0):
    """The two runs (rules.md 5) for a crossing `width` metres wide and a piece `piece_w_mm` wide
    across the deck and `piece_d_mm` deep along it, on a deck `length_mm` long, or None when the
    piece leaves no deck between the rows or no length between the ends."""
    o = round(width / 2 - piece_w_mm / 2000, 3)
    if o <= 0:
        return None
    s = round(piece_d_mm / 2 / length_mm, 6) if length_mm > 0 else 0.0
    if s >= 0.5:
        return None
    k = key_of(name)
    return [{"key": f"{k}_parapet_{side}", "asset": piece_id, "spacing": spacing, "ends": "both",
             "on": {"ref": name, "span": [s, round(1 - s, 6)], "offset": off}} for side, off in (("l", o), ("r", -o))]


def refusal(width, piece_w_mm, piece_d_mm=0, length_mm=0):
    """Why `runs` gave None: `narrow` (no deck between the rows) or `short` (no length between the
    end insets); None when it gives runs."""
    if round(width / 2 - piece_w_mm / 2000, 3) <= 0:
        return "narrow"
    if length_mm > 0 and round(piece_d_mm / 2 / length_mm, 6) >= 0.5:
        return "short"
    return None


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def read_piece(step, asset):
    """(id, width across in mm, depth along in mm) of the parapet piece, or None."""
    t = step.pack(select=asset if asset.startswith("@") else [asset], columns=["bounds"])
    ids = _list(t.get("id", []))
    if not ids:
        return None
    r = min(range(len(ids)), key=lambda i: ids[i])
    b = [int(v) for v in _list(_list(t["bounds"])[r])] if "bounds" in t else [0, 0, 0, 0, 0, 0]
    return ids[r], b[3] - b[0], b[4] - b[1]


def length_of(points):
    """A path's length in mm, from its vertices in mm."""
    pts = [(float(p[0]), float(p[1])) for p in points]
    return sum(((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** 0.5 for a, b in zip(pts, pts[1:]))


def read_deck(step, name):
    """(width in metres, length along the path in mm) of the crossing."""
    fl = step.features(select=[name], columns=["width"])
    v = _list(fl["width"])[0] if "width" in fl else None
    if isinstance(v, str):
        v = json.loads(v)
    return float(v), length_of(_list(fl.points(0)))


def generate(step):
    name = step.for_each_item
    try:
        p = params_of(step.params)
    except ParamError as e:
        step.error("params", "step." + step.id, str(e), "see lib/ways/parapet.py", explain=EXPLAIN["params"])
        return
    try:
        piece = read_piece(step, p["asset"])
    except Exception:                               # an id the pack does not hold
        piece = None
    if piece is None:
        step.warning("no_piece", "feature." + name, f"no asset `{p['asset']}` in the pack for `{name}`'s parapet",
                     "name an asset the pack holds in the step's `asset` param", explain=EXPLAIN["no_piece"])
        return
    width, length = read_deck(step, name)
    got = runs(name, width, piece[0], piece[1], p["spacing"], piece[2], length)
    if got is None:
        if refusal(width, piece[1], piece[2], length) == "short":
            step.warning("short", "feature." + name, f"`{piece[0]}` is {piece[2] / 1000:g} m deep; `{name}`'s deck "
                         f"is {length / 1000:g} m long", "use a shallower piece, or a longer crossing",
                         explain=EXPLAIN["short"])
            return
        step.warning("narrow", "feature." + name, f"`{piece[0]}` is {piece[1] / 1000:g} m wide; `{name}`'s deck "
                     f"is {width:g} m", "use a narrower piece", explain=EXPLAIN["narrow"])
        return
    for r in got:
        step.out.table("run", r["key"], asset=r["asset"], spacing=r["spacing"], ends=r["ends"], on=r["on"])
    step.info("laid", "feature." + name, f"two runs of `{piece[0]}` along `{name}`'s deck, "
              f"{got[0]['on']['offset']:g} m each side of its line", "none needed", explain=EXPLAIN["laid"],
              data={"asset": piece[0], "offset": got[0]["on"]["offset"], "spacing": p["spacing"]})
