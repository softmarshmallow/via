# Derived results

A derived result is something the build finds rather than the author writes: the land above a
height, a hollow filled to a level, a course found over the land, a line water would follow, or
what a simulation returns. This chapter says how each is written, what it reads, how it holds under
authority, and how a simulation is checked after it runs (R14).

Words are from `vocabulary.md`. Authority: `docs/doctrine.md`, chiefly R14, R16, R19(b), R23, R31
and R32. The rules this chapter follows are `authority.md` 2.3 (a shape reads the head), 3 and 4 (tiers
and order) and 7 (steps under authority); the mask is `geometry.md` 3.7; posts, post order and the
maths module are `numbers.md` 4 and 8. Everything here is M2 (`vale`, `basin`).

## 1. The forms

| Result | Written as | Shape | Made by |
|---|---|---|---|
| Where a field is above or below a level (a contour) | `where` (`geometry.md` 3.7) | region, every piece | the core |
| One hollow filled to a level | `where` with `holds` (section 2) | region, one piece | the core |
| A course found over the land | `route` on a path (section 3) | path | the core |
| A line water would follow; the level a hollow holds to | a step, through `derive.descend` and `derive.spill` (section 4) | path; numbers | a step |
| What a simulation returns | a generate step (section 5) | rows, fields, features | a step |

Each is first class: the text forms are features with an id, a kind and a tier; a step's are
features named `<step>#<key>` (`identity.md` 2.4) and selected by `{ of = "<step>", class =
"feature", near }`. Either is a place a thing is put `on`, a path `follows`, a scatter's `within`,
and an entry of the change report.

Rules for every form:

- **Head, not effects.** A derived shape reads each field's head view (`authority.md` 2.3): its
  base and every `then` row, rows a step returned included, never an effect. So it is known before
  any effect, it conforms to the rows, and no kind's entries, its own included, can move it. A shape
  that must see an effect is made by a step in a higher layer, which reads the full view
  (`authority.md` 4.4) and returns the shape as a feature. The spike's two floods that read each
  other and silently missed an effect (challenge 5) are therefore a rule, said here, not an accident.
- **Tier orders entries, not shapes.** A derived shape takes any tier (`authority.md` 3): the tier
  says how firmly its entries hold, never where its shape lies. A lake found by `holds` and written
  `anchored` holds its level over everything derived and shaped.
- **Whole extent.** It is resolved over the whole extent (`fields.md` 6.1), so no window changes it.
- **Change.** It is made again when what it read changes (`graph.md` 5.1): a moved point or end, an
  edited row, a new world `seed` that moves the land under it. Nothing else re-makes it.
- **Empty is a warning.** A form that finds nothing says so and builds (principle 4, as
  `geometry.md` 3.7). What needed the shape reports its own error.

The text forms and the operations of section 4 are one implementation: a worker's `derive.where`
and `derive.route` give the bytes of `where`, `holds` and `route` over the same view (R19b: our
libraries reach no further than a user's step).

**Contours** need no form of their own: `where = "= f('height') - 30"` is the land above 30 m, its
pieces kept (`geometry.md` 3.7), and their outlines are the contour lines. `on = { ref, t }` walks
an outline; `{ of, class = "piece", near }` picks one piece (`identity.md` 2.3).

## 2. One hollow: `holds`

`holds` takes a point, written only beside `where`. The region is then the one piece of the
`where` region that holds the point (closed, by `geometry.md`'s exact point-in-ring test), with its
holes. The other pieces are not made.

```toml
[feature.tarn]                       # a lake at a level fills whatever hollow holds the point
kind  = "lake"                       # a water library's kind, with attribute water_height
water_height = 26.4
where = "= water_height - f('height')"
holds = [338, 186]
```

- The ground decides the shore: a new `seed`, a moved hollow or an edited row moves it, and
  whatever stands `on` the lake follows (`vale`).
- No piece holds the point: the region is empty, `derive.dry` (warning), with the condition's value
  at the point.
- The piece touches the extent's outline: at that level the hollow runs out of the world.
  `derive.open` (warning), at its first vertex on the outline in post order. A `where` without
  `holds` touching the outline is normal and says nothing.
- `holds` without `where` is `derive.holds_alone`.

The level at which a hollow would run over (the spike's `level = "spill"`) is a search, not a
condition: a step finds it with `derive.spill` (4.1) and returns the lake.

## 3. A found course: `route`

`route` takes a formula, the cost per metre of travel at a point, written only on a path. The
path's written points are its waypoints: `starts_at`, every point of `path`, then `ends_at`
(`geometry.md` 2.7). Between each two in turn, the course is the least-cost way over the posts. A
typed course is a path without `route` (R16).

```toml
[feature.river]
kind      = "river"
authority = "shaped"
starts_at = "spring"
ends_at   = "haven"
path      = [[1900, 2100]]                       # a place it passes
route     = "= 1 + 40 * slope('start')"          # 'start' is the ground before the simulation
```

1. **Cost.** The formula is evaluated at every post, with the names and head view of a `where`
   (`formulas.md` 6.6). It must be above 0 everywhere: a value at or below 0 is `formula.range` at
   the first such post, since a course over free ground has no least.
2. **Ends.** Each waypoint is joined to its nearest post: least squared distance in integer
   millimetres, ties to the lower post in post order.
3. **Steps.** Each post joins its eight neighbours. A step's length `len` in metres is `r` or
   `r * sqrt(2)`, `r` the resolution in metres as a double; its weight is
   `0.5 * (c(a) + c(b)) * len`, in doubles, in that order.
4. **Search.** Dijkstra (1959) from the leg's first post: a queue ordered by distance, then post
   order; each post settled once; a neighbour's distance and predecessor change only on a strictly
   smaller distance. The leg ends when its last post is settled; the posts are read back through
   the predecessors.
5. **Simplified.** Each leg's posts are simplified by Douglas and Peucker (1973) to within half the
   post spacing, `r / 2`: the first and last post kept; of the posts between, the one farthest from
   the line through the two kept around it (ties to the first) is kept when it lies more than
   `r / 2` from that line, and each side is simplified in turn. Distances are compared exactly in
   integer millimetres. The eight-neighbour staircase becomes the posts where the course turns, and
   no kept post lies more than half a spacing from the posts it stands for (so a course that went
   round a costly post still does).
6. **Course.** The first waypoint, then for each leg its kept posts without its first and last (the
   posts nearest its waypoints, whose places the waypoints take), then the leg's end waypoint;
   consecutive repeats removed. Then, first in order and again until none is left, a point that is
   not a waypoint is dropped where the course turns by more than 90 degrees (it would double back)
   or where it stands closer than `r / 2` to a neighbour. With `smooth = true` on the feature,
   `smooth` (`geometry.md` 2.4) then smooths through these points; as for any path its default is
   `false` (principle 2). `t` is measured on the result.

Every post has a finite cost and the grid is connected, so a course always exists. Moving an end
(`haven`) finds a new course with no other edit (`basin`). The cost here reads `start`, a field the
simulation does not write, so the course is found over the starting ground and the step may read the
course without a cycle (`graph.md` 2.2). `route` without `path` is `derive.route_alone`.

## 4. Engine operations for steps (R23)

Worker methods (`protocol.md` 3.3, Computes). Each is a batch, columns in and columns out, and each
field it reads is traced as a `build.field` read of that view. `view` is `head` or `full`, default
`full`, as `build.field` takes it; a formula reads the step's view (`formulas.md` 6.6).

| Method | Params | Result |
|---|---|---|
| `derive.where` | `formulas` (str column, n rows), `holds` (optional, i32 `(n, 2)`), `view` | A feature list of n regions: 3.7's mask of each formula, kept to the piece holding its point as section 2; a bool column `dry`. |
| `derive.route` | `cost` (a formula), `waypoints` (a feature list of n paths), `view` | A feature list of n paths, each as section 3 steps 2 to 6 without `smooth`; an f64 column `cost`, the sum of the weights of the steps the search took. |
| `derive.descend` | `field` (a name or selector matching one scalar field), `x`, `y` (i32 columns), `view` | A feature list of n lines (4.2); a str column `end`, `pit` or `edge`. |
| `derive.spill` | `field`, `x`, `y`, `radius` (i32 column, millimetres, 0 for none), `view` | f64 column `level`; bool column `dry` (4.1). |

A selector matching no field, several, or a weights field is error -32602 naming `field`. Values
are the field's narrowed values (`fields.md` 1.1), compared exactly. The **start post** of a point
is the lowest of the four posts of the grid cell holding it (`fields.md` 1.2), ties to post order.

### 4.1 Spill

- **Outlets** are the posts on the extent's edge and, with a radius `R`, every post farther than `R`
  from the point (`dx² + dy² > R²`, in integers).
- **`level`** is the least, over all paths of eight-neighbour posts from the start post to an
  outlet, of the greatest value on the path, both ends included. Priority-flood finds it (Barnes,
  Lehman and Mulla, 2014); the definition, not the algorithm, is normative.
- **`dry`** is true when the start post's own value equals `level`: the point is in no hollow.
- The piece of `level - f` holding the start post then touches no outlet: every post in it is
  joined to the start post by posts below `level`, and none is an outlet. A library returns the lake
  as `derive.where` of `"= <level - freeboard> - f('height')"` holding the point.

### 4.2 Descend

The line is steepest descent over the field's **surface**: the posts joined into triangles, each
grid cell split by its diagonal from post `(i, j)` to `(i + 1, j + 1)`, a plane in each triangle.
It runs at any bearing, the continuous direction of D-infinity (Tarboton, 1997) traced as a line;
an eight-way walk from post to post (O'Callaghan and Mark, 1984) is not used, since its drainage
runs in rows and diagonals (the spike's rectilinear drainage, a recorded failure).

- **Inside a triangle** the line runs against the triangle's gradient to the side it meets.
- **On a side** it enters the adjacent triangle its gradient points into, strictly; of two, the
  steeper, ties to the triangle below or left of the side. When neither does (a groove), it runs
  along the side to its lower end.
- **On a post** it leaves by the steepest of the post's six sides to a lower post (drop per metre)
  and the triangles around it whose gradient points strictly inside them; ties to the sides, in post
  order of their far post, then the triangles counter-clockwise from east.
- Each point it reaches is rounded to the nearest millimetre along the side it lies on, a post when
  that rounds to an end. The value at each point must be strictly below the last, so it ends.
- It ends on the extent's outline where a triangle there falls strictly out of the extent
  (`edge`), and where nothing falls (`pit`). It starts at the point itself.
- The line is the point, then each point reached, consecutive repeats removed. With fewer than two
  distinct points it is returned as a point feature, `end = "pit"`.
- A line ends in a pit; whether the core continues it over a filled hollow is (open).

## 5. Simulation under authority (R14)

A simulation is a way of deriving, so it obeys authority (`authority.md` 7). This section fixes the
words and the check `authority.md` 7 leaves open.

- **A stated fact** is a feature written in text (a world, a layer or a change) whose tier is
  `shaped` or `anchored`. A derived feature asks nothing to hold, and nothing a step returns states
  anything.
- **Facts in.** The step reads stated features in its stratum through `build.features` and fields
  through the views of `authority.md` 4.4. How a fact becomes a starting condition is the library's
  knowledge (a river is a groove in the starting surface, not held cells).
- **Fields and found geometry out.** Rows on fields below it, fields of its own and features (R32,
  `protocol.md` 5). What it found (the course the water took, a shore) is a derived or shaped
  feature `<step>#<key>`, keyed by the name of the fact it came from (`identity.md` 2.2), so text
  stands on it by `{ of, class = "feature", near }`. Its rows sit before every effect, so every
  stated entry is applied after them (`authority.md` 4.1). One call, one result: the core has no way
  to make many and pick one.
- **Facts checked after.** A lint step names the generate steps it checks in `checks` (R31's queued
  key, here on the lint step):

```toml
[step.terrain]
when    = "generate"
worker  = true
command = ["python", "-m", "viasdk", "lib:land/erode.py"]

[step.facts]
when    = "lint"
worker  = true
command = ["python", "-m", "viasdk", "lib:land/facts.py"]
checks  = ["terrain"]
```

- `checks` is a list of step ids, each a generate step of this world in any layer. On a step whose
  `when` is not `lint` it is `derive.checks_when`; an id that is no generate step is
  `derive.checks_target`.
- The lint's request carries `checks` (`protocol.md` 2): per named step, sorted by id,
  `{ "step", "facts" }`, where `facts` is the sorted names of the stated facts that step read. A
  worker read a fact when its name is in the answer of a traced `build.features` read; a plain
  command read every stated fact in its stratum; under `for_each`, the union over its items. The
  member is part of the lint's static key, so the lint runs again when the facts read change.
- A generate step that read a stated fact and returned rows or features, which no lint step's
  `checks` names, is `authority.unchecked` (error, at its `when` line). The hint is a lint step with
  `checks = ["<id>"]`.
- The lint reports each fact that does not hold with its own code `step.<id>.<name>`, subject the
  fact, a position, and in `data` what would hold instead (principle 8): the level the lake keeps,
  the course the water took.

**Acceptance (`basin`).** Moving `haven` re-finds the river's course; the step, which read it, runs
again; the valley and what stands on it resettle; the lint runs again. An edit in a layer above the
step changes no answer in its trace, so it is reused (`protocol.md` 4.4).

## Diagnostics

Codes of other chapters used here: `formula.range`, `authority.unchecked`, `graph.cycle`,
`geom.mask_empty`, `geom.mask_tiny`, `geom.mask_dropped`.

| Code | Severity | M | Says | Hint |
|---|---|---|---|---|
| `derive.dry` | warning | 2 | A `holds` point lies in no piece of its `where` region, so the region is empty; the condition's value at the point. | Move `holds` into a piece, or change the condition. |
| `derive.open` | warning | 2 | The piece `holds` keeps touches the extent's outline; its first vertex there. | Change the condition so the piece closes inside; `derive.spill` gives a level that does. |
| `derive.holds_alone` | error | 2 | `holds` is written without `where`. | Write `where`, or remove `holds`. |
| `derive.route_alone` | error | 2 | `route` is written on a feature without `path`. | Give the path its ends with `starts_at` and `ends_at`, or remove `route`. |
| `derive.checks_when` | error | 2 | `checks` on a step whose `when` is not `lint`. | Move `checks` to a lint step. |
| `derive.checks_target` | error | 2 | `checks` names something that is not a generate step of this world; the nearest generate step. | The nearest generate step id. |

## Conformance

Each case runs through the command line (R20); the runner also checks that a clean build equals the
incremental one. Cases live in `spec/cases/derived/<case>/`. Unless a row says otherwise: extent
`[0, 0, 8, 8]`, resolution 1, posts named `(i, j)`. `twin` is `height` with
`base = "= min(abs(x - 2) + abs(y - 4), abs(x - 6) + abs(y - 4))"`; `bowl` is
`base = "= (x - 4) * (x - 4) + (y - 4) * (y - 4)"`.

| # | Input | Action | Must hold |
|---|---|---|---|
| d01 | `twin`; a feature `lake` with `where = "= 1.5 - f('height')"`, `holds = [2, 4]` | build | One piece; it holds (2, 4) and not (6, 4); its rings equal those of the piece holding (2, 4) of the same `where` without `holds`. |
| d02 | d01 with `holds = [4, 0]` | build | `derive.dry`, exit 0, an empty region. |
| d03 | d01 with `"= 3.5 - f('height')"` | build | `derive.open` at the first vertex on the outline. |
| d04 | `holds` without `where`; `route` on a point feature | build | `derive.holds_alone`; `derive.route_alone`; exit 1. |
| d05 | d01, a guide `on = { ref = "lake", t = 0.25 }`; a change moving `holds` to `[6, 4]` | build, then with the change | The region is the other hollow; the guide moved, its cause `feature.lake.holds`. |
| d06 | Flat cost `route = 1`, `starts_at` a point at (0, 0), `ends_at` one at (8, 0), `path = []` | build | The course is (0, 0), (8, 0): the nine posts of the row, simplified. |
| d07 | As d06, ends at (0, 0) and (2, 1) | build | The course is (0, 0), (2, 1): the search keeps (0, 0), (1, 0), (2, 1) by its tie rule, and (1, 0) lies 0.45 m from the line, within half a spacing. |
| d08 | `route = "= if(f('height') > 1, 100, 1)"` over a ridge between the ends | build | No vertex of the course stands on a post of cost 100 where a way round exists: (0, 4), (4, 8), (8, 4). |
| d09 | `route = 0` | build | `formula.range` at post (0, 0), exit 1. |
| d10 | d06; then the end point moved to (8, 4) | build twice | The course re-found; the change report's entry for it names the end's cause. |
| d11 | A worker calling `derive.spill` on `bowl` from (4, 4), radius 0, then radius 2 | build | `level` 16, then 5; `dry` false both times. |
| d12 | `derive.spill` on `twin` from (2, 4) | build | `level` 2, `dry` false. |
| d13 | `derive.descend` on `bowl` from (8, 8); then from (4, 4) | build | (8, 8), (7, 7), (6, 6), (5, 5), (4, 4), `end` `pit`; then a point feature, `end` `pit`. |
| d14 | `height` `base = "= x"`; `derive.descend` from (4, 4) | build | (4, 4), (3, 4), (2, 4), (1, 4), (0, 4), `end` `edge`. |
| d15 | A worker returning `derive.where` and `derive.route` results as features, beside the same `where`, `holds` and `route` in text, all reading `height`'s head view with no rows | build | Each returned shape equals its text twin byte for byte. |
| d16 | A generate step reading an anchored feature and returning a row; no lint | build | `authority.unchecked` at its `when` line, exit 1. |
| d17 | d16 with a worker lint, `checks = ["<step>"]` | build | No `authority.unchecked`; the lint's request holds `checks` = `[{"facts": ["<feature>"], "step": "<step>"}]`. |
| d18 | `checks` on a generate step; `checks` naming a lint step | build | `derive.checks_when`; `derive.checks_target`, nearest the generate step. |
| d19 | d17; the anchored feature moved; then a feature in a layer above the step moved | rebuild each | Step and lint ran; then the step `reused`. |
| d20 | `basin` (example); `--change move_haven` | build, then with the change | The step, the river's course and the lint ran again; the dress layer's own nodes that read nothing the step returned (its views placed by coordinate or on a stated fact the change leaves) did not. The rest of the dress layer reads the settled land, so it runs again. |
| d21 | `height` `base = "= 2 * x + y"`; `derive.descend` from (4, 4) | build | (4, 4), (3, 3.5), (2, 3), (1, 2.5), (0, 2), `end` `edge`: the line crosses cell sides between posts (4.2). |
| d22 | Flat cost, `starts_at` a point at (0.3, 0.2), a waypoint at (5, 1), `ends_at` one at (7.6, 6.7); then the same over `route = "= 1 + 0.25 * x"` | build | The course starts and ends at the written points and passes (5, 1); no turn of more than 90 degrees and no segment shorter than 0.5 m. |

## Changed from the spike

| Spike | v1 | Why |
|---|---|---|
| `flood = { field, level = "spill", fence, max_depth, freeboard }` on a lake | `where` with `holds`; the spill level from a step's `derive.spill` | One mask (`geometry.md` 3.7); a search is a step's (R7, R23). |
| `course = "find"` with an empty path | `route`, a cost per metre, through the written points | Ends and passes by name (R16); the cost is the author's. |
| Effects on a field a flood read never reached it, unsaid | A shape reads the head, by rule; a step above reads effects | `authority.md` 2.3. |
| A held river cell; a lake held by its region | Facts become conditions in the library; checked after by a lint with `checks` | R14: "a river is a groove, not held cells". |
| Drainage from post to eight-way post, in rows and diagonals | `derive.descend` follows the triangulated surface at any bearing (4.2) | The rectilinear drainage was a recorded failure. |

## Open

| Question | Decided by |
|---|---|
| Continuing a flow line over a filled hollow to its outlet, and flow accumulation (a catchment). | `basin`'s tributaries, when a library needs them. |
| Sixteen neighbours for `route`, against eight (`descend` follows the surface at any bearing, 4.2). | `basin` judged at eye height (R17). |
| `holds` naming a point feature instead of a coordinate. | The first world whose hollow must follow a named place. |
| A spill level in text, with no step. | `vale`: whether the water library's step is too heavy for a tarn. |

## Owed by other chapters

| Chapter | What |
|---|---|
| `authority.md` 7, 8, Open | 7 point 4 and `authority.unchecked`: `checks` decided here (section 5); 8: decided here (section 1). |
| `formulas.md` 6.1 | Rows for `route` (per post, above 0) and for `where` beside `holds`. |
| `geometry.md` 2.3, 3.11 | `holds` and `route` as written-shape keys; 3.11 points here (section 3). |
| `graph.md` 5.1 | `checks` and the request's `checks` member in a lint's static key. |
| `identity.md` 2.3, 2.4 | R32 closes the `field` class's open mark. |
| `conformance.md` | The case folder `spec/cases/derived/`. |
