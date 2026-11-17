# The first cut: one page for the owner's review

Branch `rewrite`, last commit `4016c85` (M7 close). Nothing is pushed; `main` has not moved.
Every number below names where it comes from; the atlas and conformance numbers are from the run
that made [`m7-sheet.png`](m7-sheet.png) (`make_m7_sheet.py`, protocol on the sheet).

## What the first cut is

The goal line (`docs/doctrine.md` 1): *Decide what matters; the world settles around it. Change your
mind, and it settles again. You never repair a map by hand.* The first cut (R27, `docs/plan.md`):
everything the spike showed, rebuilt as final examples in the v1 vocabulary on the new Rust core,
nothing of the spike's core or words carried over; Godot only.

| | What it set out to show | Challenge rows (held, partly, not shown) | Sheet |
|---|---|---|---|
| M0 | Rulings R20 to R33 recorded, the vocabulary page, SPEC 11 and 12.8 sorted, the hello step (a 6.5 MB field out and back, a killed worker) and the null digest | none | [m0](m0-sheet.png) |
| M1 | One decision settles and resettles on foot: `ford`, its hand edits kept through a rule change | 2, 2, 0 | [m1](m1-sheet.png) |
| M2 | Land and user code through one public door: a moved river mouth resettles, a new seed moves the valley, a step re-runs only when what it read changed (`vale`, `glade`, `relief`, `massif`, `basin`) | 5, 8, 1 | [m2](m2-sheet.png) |
| M3 | A town that holds on foot and under edits: the moved hill, R13's four obligations, reach as a library lint (`quay`, `townland`, `example`, `monastery`, `city`) | 2, 11, 1 | [m3](m3-sheet.png) |
| M4 | Worlds built apart compose by name: an import moved and turned, a forgotten exit says what to write, a moved square re-runs only its district (`strait`, `county`, `border`) | 3, 4, 0 | [m4](m4-sheet.png) |
| M5 | More than one surface, buildings as a library: decks, caves, indoors through a door, the town kit's three changes (`coast`, `station`, `college`, `market`, `pistol`, `sidearm`) | 0, 2, 0 | [m5](m5-sheet.png) |
| M6 | Whole and handed over: every example joined in `atlas`, chunks streamed on a walk, an over-budget chunk fails naming its asset, the player map as data | 0, 14, 0 | [m6](m6-sheet.png) |
| M7 | Outsiders build with it: four agents of two Claude model lines built `mill` and `crossroads` from the documents alone; `spike/` deleted (record: tag `spike-final`) | 0, 7, 0 | [m7](m7-sheet.png) |

The challenge counts are the rows of `docs/challenges.md` whose milestone column names that
milestone (its slices included); M1's "held" counts one "Held (points)", M6's "partly" three
"Partly (the data; styling set aside)". No milestone's rows are all held; M6's on-foot row (53)
reads not yet acceptable on foot.

## What holds and what does not

Holds (this run unless named):

- `atlas`, every example joined, builds cold in a fresh store in 241.1 s (exit 0, 4.95 GB peak,
  14716 of 15216 nodes run, load 6.5 to 15.4); its 18 base rows are in band, 7 of them banded at
  the value measured (below).
- Built again unchanged: 1.1 s into the same folder and 0.7 s into a sibling folder (nothing run),
  95.4 s into a folder at another depth (1 node run, 40% of cold).
- Changes, each `--against` the base: `move_market` 156.8 s (65% of cold), `move_square` 153.4 s
  (64%), each with its rows in band; `forgotten_exit` stops with exit 1 naming the exit and the
  line that answers it (111.3 s); `over_budget` stops with exit 1 naming chunk [17, 8] and
  `tree_oak` (63% of its triangles), after 139.5 s (58%).
- Walks streamed at 96 m with water: 97 arrive, 29 are blocked, none falls, of 126 (the M6
  sheet: 87, 33, 6 of 126); every atlas join arrives, `coast.along_cliff` arrives (59.6 m).
- Before and after (sheet page 2): `move_market` seen from `join_market`, whole, not streamed; the
  road follows the moved market and the walk arrives both times (23.5 m, then 28.7 m); 97 arrive
  and 29 are blocked before and after, no walk's outcome changed. The table's builds were not
  kept, so base and change were built again for this pair from the same tree (239.8 s and 156.1 s,
  a store of their own, 1-minute load 7.75 to 7.96).
- Conformance on the release binary (`tools/conformance`, cases at or below the milestone): 763
  passed at milestone 3, 835 at 4, 888 at 5, 911 at 6, 938 at 7 of 951 cases; 1 fails at each,
  `graph/epoch`, which reads a test-only variable a release binary ignores (its case says so);
  12 are pending by their own `pending` line (`via lock`, a path across Windows drives, others).
- Protocol for the atlas numbers: one run each, release binary of `4016c85`, default threads, one
  fresh store, macOS arm64 (Apple M4 Pro, 14 cores) shared with other agents (load 6.5 at the
  start, 14.1 at the end); the sheet gives the load per build.
- The outsiders (docs/challenges.md row 51): all four worlds built; first build 9 and 5 min
  (Fable `mill`, `crossroads`), 7 and 4.5 min (Sonnet, second run); 31 trips in the first run, 22
  in the second. First run: every document and message trip fixed, and its two engine faults and
  the loader's `load.bad_file`, each fault with a case that fails on the old binary. Second run:
  the fixes row 51 lists (lib/water's `weir` with cases that fail at the old radii, the SDK's
  kindless feature, a step's report on an import's feature, the loader's `along`, an absolute
  import shown climbing; some by case, some by a unit, SDK or loader test, and notes in the
  documents it names); its open trips are below.

Does not hold (`TODO.md` "From M7 slice 1" and "From M7 close"):

- Atlas rows banded at the value measured (NOT HOLDING in `expect.toml`), this run: grade_max and
  road_join_max 0.442 (target 0.25), join_step_max 37.24, road_painted_in 0.286, bookmarks_wet 2,
  exit_in_grade_max 1.122, exit_out_grade_max 0.375. The places stand on plinths and in pits
  where roads join.
- `coast.in_cave` is blocked in atlas (this run: after 14.6 m by `coast.east_face.face`; 1.95 m
  short in the M7 close review); `city.old_town_street` parts streamed from whole
  (1 mm at 14 m, a house corner at 101 m).
- `over_budget` is counted after the per-chunk nodes: 58% of cold this run, target 30%. A build at
  another depth still costs 40% of cold with one node run (95.4 s this run; 85.0 s and 153.1 s in
  the M7 close reviews).
- `basin`'s ground_tarn_view 44.36 against the oracle's 57.35 +-6, ground_middle 55.79 against 62.38 +-6.
- From the outsiders' second run, open messages and trips: `formula.outside_field` names the
  library's line, not the moved import; `via build --help` is not understood; knock-on errors
  after a misspelt name; `lay` beside its building's step is quiet on the console ("0 errors, 0
  warnings"); an import's own warnings unfolded; the weir draws no mesh. Open gaps from both runs:
  a building placed `on`/`face`, `lay` on a turned outline, a view on a produced storey, a lift at
  a named place, a building's own yard or pad.
- Windows is unproven; both outsider lines are Claude models.

## Rulings open to the owner

Each is recorded in `docs/doctrine.md` section 5 and marked open there (in part, for R39, R41, R42).

| # | The question |
|---|---|
| R39 | Which surface a thing stands on, and who is told when it does not fit (owner first, two items: a point laid off its deck by more than a lattice unit says nothing; a step's returned instance is never tested, a library's own instances are its lint) |
| R41 | Passable ground and what a walk may assume (owner first: the last sentence; the loader's `load.bad_target` against a checked name attribute) |
| R42 | Things with parts, states and accessories (a part's height as its asset's negative `sink`) |
| R44 | Budgets: who declares a limit and how it blocks |
| R45 | Export steps: where their files go and what they may be called |
| R46 | Assets an import brings |
| R47 | The player map's fixed rules |
| R48 | Streaming: what a streamed reader must equal |
| R49 | An import's kind surfaces, cut and meetings |
| R50 | Water and ground at joins |
| R51 | Where a parent keeps a change it asks of an import |
| R52 | A library step and its kinds file |
| R53 | The cut band of a narrow brought strip |
| R54 | An untrusted step's committed program digest |
| R55 | What a build's leaves are keyed by |
| R56 | Naming a walk's stop |
| R57 | How diagnostics word what they name |
| R58 | A place on a turned import's edge |
| R59 | Library choices from the outsider runs (`STUB_MIN`, `end_unjoined`, `low_deck`) |
| R60 | lib/water's `weir` and its effect radii |
| R61 | Where the loader reads a returned bookmark's `along` |
| R62 | A step's diagnostic on an import's feature |
| R63 | How an absolute import's path is shown |

R20 to R38, R40 and R43 are not marked open.

## Decisions owed to the owner (from `TODO.md`, `docs/doctrine.md` 6 and `docs/plan.md`)

A selection, as of `4016c85`: the decisions that bear on the first cut; the rulings above are not
repeated. Other open TODO items name the owner too: `raise_max` and quay's hill-street rows, the
kit rename `ends` to `caps`, a bridge over a sheer face as `geom.unowned_crossing` (geometry.md
2.7), `along` as text (build.md 15.1), fitting other map examples and design reference matching
(both left for the M0 sheet), and the "decided, open to the owner" lines under each pass.

- **The place lift trade at joins.** Atlas's places stand on plinths and in pits where roads meet
  them (grade_max 0.442 against 0.25): take a lift at each exit, or fit the plain to each rim.
- **R48's streamed walk bound.** A bound for walks that graze a corner, or one body order for whole
  and streamed (`city.old_town_street`).
- **The stand-in licence.** A redistributable asset pack so example renders can be committed
  (plan, "Comes back to you later"); until then renders stay out of the tree.
- **Windows proof.** It needs the branch on hosted runners, which needs a push; Linux and Windows
  testing were set aside by the owner on 2026-11-13 (doctrine section 7).
- **Another vendor's outsider run.** Both lines run so far (Fable, Sonnet) are Claude models.
- **`spike/local`, `spike/out` and `spike/build`.** Still on disk, gitignored, after `spike/` left the
  tree: may they be deleted?
- **Stations by distance from one end** (doctrine 6, "Open (M5 close)"; TODO "Stations by distance
  from one end"): stations at `k L / n` slide every lot when a stretch shortens; by distance from
  the start (recommended) moves only the last cells and changes every cells case's numbers. The
  pending case `identity/c27_stretch_lengthened` waits on it.
- **A layer's `then` rows on a field declared below it** (TODO "A layer cannot write a row on a
  field another document declares"): allow them at the layer's rank (recommended), or keep atlas's
  road-paint step.
- **An import's produced instances give way to the parent's `keep_out`** (TODO "A parent's way does
  not clear an import's scatter"): proposed for composition.md, with one `import.gave_way` per
  import; atlas routes round the wild meanwhile.
- **The far stand-in pop bound** (TODO "Far stand-ins pop at the load edge"): 11 of 123 atlas views
  over 1% whole against streamed (as recorded in TODO); shaded or rounder far trees, or a looser bound past the load radius.
- **The bridgehead height gap** (TODO "Fresh agent (`weir`)"): a library rule or a core tolerance;
  built at M7 close as lib/ways' `end_unjoined` check (decided, open to the owner).
- **The dogfood map** (plan, "Comes back to you later"; TODO "Pick the dogfood target"): `county`
  proposed, yours to redirect.

## How to review

- The commits on `rewrite` since `main`: `git log --oneline main..rewrite | wc -l` gives 32 (the
  30 below, the commit that adds this page and a cleanup commit).
  By milestone: the record and M0, 6 (`56cb6a2` terrain patch, `eb154b5` the reboot record,
  `29dc21a`, `a14d1cf`, `562cd90`, `92b2a00`); M1, 3 (`bae0728`, `c4e5a6c`, `0e522fe`); M2, 2
  (`5013e17`, `2cf9798`); M3, 5 (`b83f135` to `eb8af62`); M4, 4 (`5a87524` to `f295b7b`); M5, 4
  (`8d2d566` to `db28af7`); M6, 3 (`96cab3a`, `b8a4e66`, `0b765be`); M7, 3 (`58febc6`, `ff13410`,
  `4016c85`).
- Read the sheets in order, then `docs/challenges.md` and `TODO.md` for what is open.
- `main` moves when the owner merges `rewrite` into it (a merge commit or a fast-forward keeps
  the milestone commits; a squash would lose them).
- The sheet's scratch data (its `--keep` folder and the outsiders' runs) is deleted: `--draw-only`
  cannot redraw it; a full run of `make_m7_sheet.py` makes it again, without the outsiders' views.
