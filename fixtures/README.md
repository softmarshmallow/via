# Fixtures

What the spike produced, kept as an oracle for the rewrite. The spike was the throwaway Python
compiler of the concept phase; it was deleted at M7 and its record is the tag `spike-final`
(commit `30dff13`). These files are data exported at that tag: what it said and measured for every
world it could build. Paths inside them (`spike/worlds/...`) are recorded as the spike wrote them
and name files at the tag.

## The rule

| Compared | How |
|---|---|
| Exit status | Exact agreement. |
| Diagnostics on a bad edit (an overlay whose exit status is 1) | Exact agreement on the set of `{severity, code, subject}`. `where` within a few metres. |
| Everything else: counts, water measures, street form, budgets, timings, kernel areas | Compared as measures. A difference is read and explained, not fixed by tuning. |

The rewrite is never optimised to these numbers (R15: a measure informs; it is never a target).
The spike was wrong in places (see `spike-final:spike/SPEC.md` section 11). Where the rewrite is right and
the spike was wrong, the fixture is updated and the reason is written in the commit.

Diagnostic codes are the spike's words. Where the v1 vocabulary renames a noun, the comparison
maps the old code to the new one; the fixture keeps the old word. (open) The code map is written
when the rewrite's diagnostics exist; M1 decides it.

## `oracle/`

One folder per build. `<world>/` is the world as written. `<world>@<stem>/` is the world with the
overlay `changes/<stem>.toml` (or, for `pistol`, `accessories/<stem>.toml`) merged over it.
`pistol@accessories.all/` merges all three accessories. `<world>@variant.<name>/` is the world
with `[variant.<name>]` merged over it.

| File | Holds |
|---|---|
| `summary.json` | The command, exit status, error, warning and info counts, `compile_seconds` (the compiler's own line), `wall_seconds` (including `uv` start-up), the machine (macOS arm64, one run, not a benchmark), instances per asset, features per kind. |
| `diagnostics.json` | Every diagnostic as `{severity, code, subject, where}`, sorted. Message and hint text are left out: they are prose, not contract. |
| `measures.json` | `report`: the build's `report.json` (water bodies). `main_ways` and `street_form`: what those steps wrote, by step id, where a world runs them. `budgets`: chunk size, chunk count and totals. `bookmarks`: id, `eye` and `look` (the point looked at). |
| `changes.json` | Only for overlays and variants: what the build placed differently from the base build (`against`), by name: added, removed, moved, swapped, by rule. |

A build with no world (an uncaught failure) has `failure` in its summary and no counts.

## `kernel/townland/`

The generated linework of `townland`, for testing the geometry kernel without the rest of the
compiler. Coordinates are metres in the world's frame (x east, y north), with no CRS.

| File | Holds |
|---|---|
| `streets.geojson` | Every feature of kind `street` (the main-way skeleton and the district streets), as lines. |
| `wards.geojson` | Every `ward`, as polygons. |
| `river.geojson` | The river's found course, as one line. |
| `lots.geojson` | The two squares (kind `lot`, polygons) and the lots step's frontage runs (kind `frontage`, two-point lines with the side they face). |
| `reference.json` | Computed with shapely (versions recorded in the file): the street lines noded with `unary_union`, then `polygonize`; face count, total area and every face's area, sorted. The river course buffered at 8.5 m and 17 m (`quad_segs=16`, round caps and joins): validity, type and area. |

## How it was made

At the tag `spike-final` (a worktree of it; the spike is not in the tree since M7):

```
cd via
cargo build --release -p via                      # the old terrain binary; basin and townland need it
cd spike
uv run python -m worldc worlds/<world>/world.toml [check.py's lint flags] \
    [--overlay worlds/<world>/changes/<stem>.toml | --variant <name>] --out oracle_<name>
```

The exact command for each build is in its `summary.json`. Lint flags follow the spike's `check.py`
(`--lint reach` for college, coast, example, monastery and border; `--map` for coast). An overlay
build starts from the base build's `placed.json`, so its `changes.json` is against the base.
Generate steps reuse their cached output under `gen/` when their inputs are unchanged, so
`compile_seconds` for `basin` and `townland` excludes most of the terrain simulation. The exit
status and error and warning counts of the fifteen `check.py` cases matched `golden.json`. The
export script read each build's `diagnostics.json`, `world.json`, `report.json` and
`budgets.json`, the step outputs under `spike/worlds/<world>/gen/` and the lint outputs under
`spike/build/.steps/`, and replaced any absolute path with one relative to the repository. The
kernel fixtures were read from the `townland` build's `world.json` and
`spike/worlds/townland/gen/lots/layer.json`. Both export scripts were one-off and are not kept.

## Not here

| What | Why |
|---|---|
| Worlds built from material outside the repository | They cannot be rebuilt by anyone else; none were found at export. |
| `zoo_*` worlds | Generated by a script, not written; they hold absolute paths. |
| `strait/a`, `strait/b`, `border/wild` | Parts of `strait` and `border`, built only through them. |
| Pictures, fields, meshes | Bytes that depend on library versions. The measures stand in for them. |
