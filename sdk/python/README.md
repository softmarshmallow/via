# viasdk: writing a step in Python

`viasdk` is the Python side of the step protocol (`spec/protocol.md`, `via-step-1`): pure Python,
NumPy its one dependency, never linking the engine. Every library under `lib/` speaks it and
nothing else; a step of your own does the same. The contract is `spec/protocol.md` sections 3 to 9;
this page is the short way in. If the two disagree, the chapter wins and this page is fixed.

## Setting up

The engine puts the SDK on the interpreter's path for each call (`protocol.md` 8.1): nothing is
installed into it. You name a Python 3.11 or later that has NumPy, once:

```sh
uv sync --project sdk/python --python 3.12                      # makes sdk/python/.venv (3.12, NumPy)
via config program python $PWD/sdk/python/.venv/bin/python
```

Any Python 3.11+ with NumPy will do (`python -m venv v && v/bin/pip install numpy`). Without one,
a build that reaches a Python step says `protocol.no_program`. A step you write is run only once
you `via trust` the world (`protocol.md` 7).

## A step

A world declares it; the script defines the function named after `when`:

```toml
[step.grades]                         # a lint over every road
when    = "lint"
worker  = true                        # the same script runs plain without it (protocol.md 8)
command = ["python", "-m", "viasdk", "steps/grade.py"]
params  = { every = 4, limit = 0.15 }
```

```python
# steps/grade.py: the steepest grade along each road, sampled every `every` metres
import math

EXPLAIN = {
    "steep": "A road rises or falls more steeply than `limit` between two samples `every` metres "
             "apart. Reroute it, grade the ground, or fix the join it crosses (an import's `lift`).",
}

def lint(step):
    every = float(step.params.get("every", 4)) * 1000        # millimetres, as the geometry is
    limit = float(step.params.get("limit", 0.15))
    (h,) = step.fields(["role:surface"], window=[0, 0, 1, 1]) # only to learn the surface's name
    roads = step.features(select="kind:road")
    for f, name in enumerate(roads.names):
        pts = [tuple(map(int, p)) for p in roads.points(f)]   # (n, 2) millimetres
        s = resample(pts, every)                               # see "Sampling along a path"
        z = step.eval(f"= f('{h.name}')", x=[round(p[0]) for p in s], y=[round(p[1]) for p in s])
        worst, at = 0.0, s[0]
        for i in range(1, len(s)):
            g = abs(z[i] - z[i - 1]) / (math.dist(s[i - 1], s[i]) / 1000)
            if g > worst:
                worst, at = g, s[i]
        if worst > limit:
            step.warning("steep", "feature." + name, f"`{name}` climbs {worst:.3f} at its steepest",
                         f"reroute it, or raise `limit` ({limit:g})", data={"grade": round(worst, 4)},
                         position=(at[0] / 1000, at[1] / 1000), explain=EXPLAIN["steep"])
```

`examples/ford` has lanes and no road, so there this lint finds nothing: try it with
`select="kind:lane"` and a small `limit` (`0.01` warns on both lanes).

## What bites

- **Units.** Feature geometry is millimetres, in and out: `step.features(...).points(f)` gives
  millimetres, and `step.out.feature(..., region=...)`, `path=` and `at=` take millimetres.
  `step.eval(formula, x, y)` takes millimetre integers. A diagnostic's `position` is metres.
- **`explain` is required.** The first `step.warning`, `step.error` or `step.info` of a code in a
  call carries its `explain` text; without it the build says `protocol.no_explain`. Keep one
  `EXPLAIN` table and pass its entry every time. `hint` is a required argument too.
- **Sampling along a path.** A routed path (`route`, `smooth = true`) has vertices about 0.25 m
  apart; reading a grade per segment divides a few centimetres of noise by a quarter metre. Resample
  by arc length first. The SDK computes nothing itself (`protocol.md` 8), so the helper is yours:

  ```python
  def resample(pts, every):
      out, want, done = [pts[0]], every, 0.0
      for a, b in zip(pts, pts[1:]):
          seg = math.dist(a, b)
          while seg > 0 and want <= done + seg:
              u = (want - done) / seg
              out.append((a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u))
              want += every
          done += seg
      if math.dist(out[-1], pts[-1]) > every / 2:
          out.append(pts[-1])
      return out
  ```

- **What an import brought.** `step.features(...)` lists an import's features too, under dotted
  names (`college.approach`). A diagnostic on one (`college.approach` or
  `feature.college.approach`) lands on the import, `import.college`, with `approach` in
  `data.place` (`spec/diagnostics.md` 6): the import's text, fixed there, not in this world. To
  lint only this world's own, filter them out (`"." not in name`) or select with `within=`. A path written with `route` is routed before a step reads it: a generate step at
  its layer or above reads the routed points (M7 close, `crossroads`).
- **A feature of no kind.** `step.out.feature(key, at=...)` with no `kind` writes none: a point or
  path a walk or a view names (a kindless path raises no `geom.unowned_crossing`). An empty kind
  is `authority.unknown_kind`.
- **Names in the returned document.** Inside it, `starts_at`, `ends_at`, `on.ref`, a kind's `names`
  attributes and `surface` name the document's own keys first (`protocol.md` 5), and the Godot
  loader reads a bookmark's `along` the same way: write `along = "walk"` for the key `walk`.
  What the step returns is seen by a step a layer above it, never by another step of its layer.
- **A step that reads stated facts.** A generate step that reads features the text states (a
  weir, to place a building beside it) needs a lint that names it in `checks`, or the build says
  `authority.unchecked` (`authority.md`).
- **Seeing what it said.** `via explain step.<id>.<code> WORLD` prints a step's `explain` text from
  the world's last build (add `--out DIR` if it built elsewhere).
- **Reference.** Every call and its arguments: `protocol.md` 8's table, and the docstrings in
  `viasdk/_step.py`. A worked generate step: `protocol.md` 9. Steps that use most of it: `lib/`.
