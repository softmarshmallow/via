"""The water library's fill step: each tarn becomes a lake filled to where its hollow would run over.

A generate step, run as a worker through the SDK, declared in the world (protocol.md 1):

    [step.fill]
    when    = "generate"
    worker  = true
    command = ["python", "-m", "viasdk", "lib:water/fill.py"]

The level a hollow holds to is a search, not a condition (derived.md 2), so it is this step's: the
engine finds it with `derive.spill` (derived.md 4.1), one batch for every tarn (R23). The step then
returns each lake in its text form, `where` with `holds` (derived.md 2), so the core makes its shape
from the same view, by the same rule as a lake an author writes, and the shore follows the ground
whenever the ground changes.

For each feature of kind `tarn` (lib/water/kinds.toml) in the step's stratum:

- `level` is the spill level of the terrain's head view (authority.md 4.4) at the tarn's point,
  looked for within `fence` metres of it when `fence` is above 0;
- the water stands `freeboard` below it, and no deeper than `max_depth` over the ground at the point
  when `max_depth` is above 0;
- the step returns `[feature.<tarn>_lake]`, kind `lake`, with `water_height` that level, rounded
  to the millimetre, `where = "= water_height - f('<terrain>')"` and `holds` the tarn's point.

A tarn whose point lies in no hollow (`dry`), or whose water would stand at or below the ground
there, returns no lake and says so: one warning, code `step.<id>.dry`, at the point, with the level
it found. Nothing else is reported: the lake's own shape reports `derive.open` when the level runs
out of the world (derived.md 2).

The step reads the terrain by `role:surface` and names it in the formula by the name the build
gives it, since a formula reads fields by name (formulas.md 4). It reads the head view: a lake
fills the land the base and the rows made, and the cut of a lane or a pad never empties it.

Library code imports only viasdk and the standard library (R19b); every number is the engine's
(R23), and the arithmetic here is two subtractions in Python doubles.
"""

EXPLAIN = {
    "dry": "A tarn asks for a lake filled to where its hollow would run over (lib/water). Its "
    "point lies in no hollow of the terrain's head view, or the water would stand at or below the "
    "ground there after `freeboard`, so no lake is returned. `data.level` is the spill level found "
    "at the point and `data.ground` the ground there, both in metres.",
}

HINT_DRY = ("Move the tarn into a hollow, lower its `freeboard`, or write a `lake` with a stated "
            "`water_height`, `where` and `holds` instead.")


def generate(step):
    tarns = _tarns(step)
    if not tarns:
        return
    (probe,) = step.fields(["role:surface"], view="head", window=[0, 0, 1, 1])
    terrain = probe.name
    xs = [t["x"] for t in tarns]
    ys = [t["y"] for t in tarns]
    radius = [max(0, int(round(t["fence"] * 1000))) for t in tarns]
    spilt = step.spill(terrain, xs, ys, radius=radius, view="head")
    ground = _list(step.eval(f"= f('{terrain}')", x=xs, y=ys, view="head"))
    levels = _list(spilt["level"])
    dry = _list(spilt["dry"])
    for t, level, is_dry, g in zip(tarns, levels, dry, ground):
        for rec in lake(t, float(level), bool(is_dry), float(g), terrain):
            if rec[0] == "lake":
                _, key, attrs = rec
                step.out.feature(key, "lake", **attrs)
            else:
                _, name, data, pos, message = rec
                step.warning("dry", "feature." + name, message, HINT_DRY, data=data,
                             position=pos, explain=EXPLAIN["dry"])


def lake(t, level, dry, ground, terrain):
    """What one tarn gives, as records: ("lake", key, attributes) or ("dry", name, data, position,
    message). Pure, so the rule is tested without an engine."""
    pos = [t["x"] / 1000.0, t["y"] / 1000.0]
    water = level - t["freeboard"]
    if t["max_depth"] > 0:
        water = min(water, ground + t["max_depth"])
    water = round(water, 3)
    if dry or water <= ground:
        data = {"level": round(level, 3), "ground": round(ground, 3)}
        why = "lies in no hollow" if dry else f"would hold water no higher than {water} m"
        return [("dry", t["name"], data, pos,
                 f"Tarn {t['name']} {why}; its hollow runs over at {round(level, 3)} m and the "
                 f"ground there is {round(ground, 3)} m.")]
    attrs = {
        "water_height": water,
        "where": f"= water_height - f('{terrain}')",
        "holds": pos,
    }
    return [("lake", t["name"] + "_lake", attrs)]


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _tarns(step):
    """Every tarn in the stratum, in name order: name, point (mm) and its three numbers, each its
    own or the kind's default (lib/water/kinds.toml)."""
    fl = step.features("kind:tarn", columns=["freeboard", "fence", "max_depth"])
    names = _list(fl.get("name", []))
    if not names:
        return []
    starts = _list(fl["ring_starts"])
    rings = _list(fl["rings"])
    pieces = _list(fl["pieces"])
    coords = [tuple(int(v) for v in c) for c in _list(fl["coords"])]
    out = []
    for f, name in enumerate(names):
        first = starts[rings[pieces[f]]]
        x, y = coords[first]
        rec = {"name": name, "x": x, "y": y}
        for k, default in (("freeboard", 0.35), ("fence", 0.0), ("max_depth", 0.0)):
            col = fl.get(k)
            has = fl.get(k + "?")
            if col is not None and (has is None or _list(has)[f]):
                rec[k] = float(_list(col)[f])
            else:
                rec[k] = default
        out.append(rec)
    return sorted(out, key=lambda r: r["name"].encode("utf-8"))
