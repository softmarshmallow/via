# The kit plan's last review

K10's second half (`docs/plan-kits.md`): what a kit's author and a world's author had to write,
what should be shorter, the archetypes and the SDK after the plan, and challenge 33. Written
2026-11-24 by an agent that did none of the work. K10's first half (`examples/borrowed`,
Kenney's Modular Buildings 2.1) is uncommitted and under review: its figures may move slightly.
Nothing was built for this review; every figure is from the committed text or a recount below.

**Protocol (ours).** Pack figures are `examples/borrowed/measure.py --packs` (non-blank,
non-comment lines; numbers outside comments, each array element; tags; `shared` keys; `front`
lines). This review recounted `examples/kit/packs/plain.toml` by hand (86 lines, 38 numbers, 20
tags: the same) and every pack with `tomllib`, which found one fault of the script (works,
below). "Derivable" compares each stated `params.pitch`, `params.span` and `sink` with the box in
the pack's `.facts.toml`, within 1.5 mm; in a kit laid by posts a panel's pitch is its box plus
the post's width (`check.py`'s own `kit_pitch`). World figures count numbers in `building`,
`row`, `court` and `pipe` tables of each world and its layers; the totals match the borrowed
README's (27, 52, 28, 17, 22, 61, 42).

## 1. What a kit's author writes

| Pack | Assets | Lines | Numbers | Pitch and span the model gives | `sink` = own height | Declared footprint | Mount numbers | Shared facts |
|---|---|---|---|---|---|---|---|---|
| `kit` plain | 14 | 86 | 38 | 12 of 12 | 1 of 1 | 12 | 0 | 8 |
| `block` city | 15 | 91 | 29 | 12 of 12 | 1 of 1 | 0 | 0 | 9 |
| `village` | 16 | 105 | 71 | 11 of 12 | 5 of 5 | 36 | 0 | 9 |
| `temple` order | 12 | 72 | 21 | 10 of 10 | 3 of 3 | 0 | 0 | 5 |
| `court` timber | 17 | 101 | 30 | 15 of 15 | 3 of 3 | 0 | 0 | 9 |
| `works` | **23** | 196 | 85 | 10 of 10 | 2 of 2 | 0 | 51 | 13 |
| Kenney, handed over | 10 | 55 | 12 | 8 of 9 | 0 | 0 | 0 | 3 |
| Kenney, laid | 11 | 67 | 18 | 10 of 10 | 0 | 0 | 0 | 5 |

Of the 292 numbers in the seven packs a kit is laid by, 80 are a pitch or span the model already
gives and 15 a `sink` equal to the piece's height (every floor tile and base piece): a third of
what kit authors typed was copied off a box. The one stated span that is a real claim is the
village valley's (3.3 m against a 4.5 m box). The borrowed README says works has 39 assets: its
script counts the 16 `[asset.x.mount.y]` tables as assets; it has 23.

Worked out by hand rather than read: the plain kit's stair (lane, landing and run sized to the
1.5 m tile; its storey first stated wrongly, caught by `kit_stated`); the declared footprints of
inside corners and the valley (48 numbers in two kits); the valley's 1.2 m reach; every
mount's `at`, `face` and `rise` in works; and in Kenney's kit the three quarter turns (read off
which quadrants of each model hold geometry), the doorway (0.25 m, off the leaf's box), and the
slope's span by its box, not its block. The grey kits' models are a further 1,173 lines of
`make_kit.py`, a modeller's work, not counted.

What repeats: `taxon = ...` on every asset (108 of 718 lines, one or two taxa a pack), `file =
"<id>.glb"` on 94 of 97 grey assets, a pitch on every wall piece.

Proposed, each with what it removes and costs:

- **Pitch and span from the model.** `lay.py:1196`, `row.py:132` and `court.py:143` already fall
  back to the box when no pitch is stated; the packs state it anyway, 67 of 67 times. Make every
  reader default the same way (a panel by posts: box plus post) and drop the restatements: 80
  numbers. Cost: R69's second claim then exists only where written, so a re-exported filler is
  said by `asset.revised`, and by `kit_unit` where the kit states a unit, not by `kit_pitch`.
  Small.
- **A plan measured from geometry, not the box.** `kit_corner` reads boxes (`check.py:551-558`);
  Kenney's corner is a symmetric 1 by 1 box, so a corner turned a quarter wrong is laid with no
  word, and the three turns the author read off the files are the one input nothing checks. An
  outline measured at `via pack accept` (orthogonal, from the triangles in plan) would check
  them and replace the 48 declared footprint numbers. Cost: a core change to `spec/assets.md` 3, and a
  fact that moves with every re-export. Medium.
- **Mounts from named nodes in the model.** 51 of works' 85 numbers are mounts, and
  `kit-contract.md` records that a `rise` restated a metre low moved the mount and not the hole
  (`low`). Cost: core, and models must carry empties (Kenney's do not, so stated mounts stay).
- **A pack's default taxon.** 108 lines. Cost: `taxon` joins the keys `defaults` takes
  (`spec/assets.md` 2.5).
- **Unknown params are said.** `kenney.toml:70` (and `sill.toml`, `revised.toml`) still writes
  `bay = true` and its header (lines 15-16) cites a door rule the README says was "built and
  taken out again"; no library reads `bay`, and nothing says so. A `kit_param` in `check` from
  each role's keys (pitch, reach, span, angle, rise, height). Small.

`sink` repeats but is a line either way; no change proposed.

## 2. What a world's author writes

| World | Features | Numbers in them | Restating the kit | Library default written | Plinth 0 for a kit with no base | Door on the side's middle | Restated, all |
|---|---|---|---|---|---|---|---|
| `kit` | 2 | 27 | 2 | 2 | 0 | 0 | 4 |
| `block` | 4 rows | 52 | 8 | 0 | 0 | 0 | 8 |
| `village` | 2 rows | 28 | 2 | 0 | 0 | 0 | 2 |
| `temple` | 1 | 17 | 2 | 1 | 0 | 1 | 4 |
| `court` | 1 court | 22 | 2 | 0 | 0 | 0 | 2 |
| `works` | 2 + 2 pipes | 61 | 2 | 5 | 2 | 3 | 12 |
| `realm` | 4 | 69 | 6 | 1 | 0 | 1 | 8 |
| `borrowed` | 3 | 42 | 3 | 1 | 3 | 0 | 7 |
| **All** | | **318** | **27** | **10** | **5** | **5** | **47** |

"Restating the kit": `storey_h` or `ground_h` equal to the kit's `shared.storey` (23), `plinth`
equal to `shared.base` where the base is the podium (4). "Library default": `storey = 0` on a door
or bay, which is the default and the only value allowed (`lay.py:2555-2558`). The examples
teach it, so authors copy it.

The examples' own "written" figures are four protocols: block counts buildings and rows (52),
works adds its instances (64), realm and borrowed the whole world (141, 67). Recounted by one
protocol (all `[...]` tables, world, layers and `walls.toml`): kit 46, block 70, village 49,
temple 34, court 38, works 88, realm 135, borrowed 67.

Worked out by hand: every lone building's outline is a sum of bays and corners, and the comments
show the sums ("four bays by three, and a post: 24.4 by 18.4 m", works; "its length less the
corner block at each end", borrowed). Where the library takes bays (row lots, court halls) the
sums vanish: temple 136 numbers by hand to 17, court 63 to 22, block 192 to 52. A block's side
rows still start where the author summed. Every world restates two or three step tables because
a library declares no step (`kinds.toml:6`), and `check`'s `params.kits`
names again the kits its buildings name (realm: four). Reusing three kits in `realm` took a
115-line script that merges packs.

Proposed:

- **The storey's height from the kit.** Filed since K6 (`TODO.md`, "What a kit states, where it
  is not yet read"; `kit-contract.md`, "Not in the contract yet") and not built through K7 to
  K11, while 23 numbers restated it (19 `storey_h`, which the kit's first height would give; 4
  `ground_h` stay). The kind's default `storey_h = 3.2` (`kinds.toml:40, 71,
  91`) hides whether a building wrote one, so it must go before `lay.py:2471` can take
  `shared.storey`. Cost: a building's height follows its kit when the kit is swapped. Small.
- **`shared.plinth`.** The floor's height over the ground the kit is made for: 9 numbers. It
  cannot default to `base`: the village's base reaches 1.5 m and its plinth is 0.3. One line a
  kit. Small.
- **A door with no `at` on the side's middle**, the only place a door on posts may stand (R74):
  5 numbers. And drop `storey = 0` from the examples and the README samples: 10 numbers, no code.
- **A lone building in bays** (`bays = [4, 3]` at a point, facing), as `court.py:195` already
  sizes halls. Cost: a second way to write a rectangle. Medium.
- **`check` reads the kits off the features it judges** (it already reads `kit`,
  `check.py:942`). Small.

## 3. The archetypes after the plan

| Archetype | Held | Added | Open |
|---|---|---|---|
| Asset | Laws 1-4 on eight kits, Kenney's included with five facts and three turns | Law 2's two claims (R69), laws 5-6 (R66), `unit` (R72), `eaves` (R73), a mount's `rise` (R75) | Scale (Kenney is a quarter of a person's size); a kit a second world can name (realm); dressing proud of a bay; turns unchecked (above) |
| Linear placement | Blocks as wall pieces with no rule added (Kenney) | Laws 5-6 (R67, R68); posts as two runs; a step's run does not give way to its own instance (R64) | Variants of filler, corner, door; neighbours across a door or corner |
| Grammar | Storeys by kind and height, base, set-back, colonnade, roofs on rectangles and an L | Law 5 (R67); symmetry ruled not an archetype (R74) | T, U, two depths, hip over an L, layered roofs, a seam piece |
| Area partition | Floors of whole tiles | Law 5 (R67) | A block as one statement; a yard as a place; a roof's region beyond its outline, known to no rule of the core (R73) |
| Meeting | Doorways, ports, rings, stacked stairs | Law 5 (R70, R71); the joint (R76); the order of shaped entries (R77, a library convention) | A joint across an import; which piece of a ring is wrong |
| Profile | The plot | R77 | A row is not levelled; moving `realm`'s village left ground 0.70 m over a floor, unsaid |
| Representation switch | Platform, pipe legs as surfaces | Two surfaces at one height are one level (R70) | A surface whose height cannot be read on plans |
| Placement | Pins on upper floors (R65) | `at = [x, y, z]` | A chest in a house follows its point, not the house (village) |
| Measure | `kit_*`, `plot_lost`, walks | | Closure (walls open, roof covered) is measured only in eight example scripts, 2,920 lines, `_npy` copied eight times |
| Markup | `poi`, `area`, the player map | | |

No pattern needed a new archetype. One was filed under none: the block's party walls ("not
Asset, not Placement: a relation"). Meeting's own definition takes it: a form that is neither
side's, one owner (`row.py`), a stated side that gives way (the windows). It should be filed
there.

Grammar is doing too much. Of 57 rows in seven "Pattern against archetype" tables (second
column, by name), it is cited in 17, for three things: a facade rewritten into storeys (its
definition), a roof decided by a whole plan ("Grammar over Area", five rows, never defined in
the ledger), and a generator that returns whole buildings or complete pipes (court's mirrored
wing, works' "no loose ends"). Area partition, which owns the block, the yard and the roof's
region, is cited once and holds most of what is open. The ledger should define "over Area" in
"Telling two apart" and say whether a generator returning a fixed set is Grammar or Recipe.

All fourteen rulings, R64 to R77, are still "open to the owner".

## 4. The SDK

| What each step does by hand | Evidence | Shortest change | lib/town |
|---|---|---|---|
| Reads a kit's rows and decodes their JSON | Six copies: `lay.py:3004`, `row.py:396`, `court.py:484`, `pipe.py:429`, `check.py:969`, `lib/town/kit.py:469`; then the first non-empty `shared` (`row.py:120`, `court.py:131`) | `viasdk`: `step.kit(taxon)`, pieces with tags, bounds, params decoded and the taxon's `shared` once | Yes: `kit.py:470` asks no `shared`, which "the kit's facts in lib/town" waits on |
| Folds a feature's attributes past their `?` null columns | `lay.py:2961-2964`, `row.py:389-391`, `court.py:433-435`, `pipe.py:422-424`, `check.py:958` | `FeatureList.attrs(f, names)` | Yes |
| Reads the ground at points | `step.fields(["role:surface"], window=[0, 0, 1, 1])` then `eval(f(name))`: `lay.py:2971, 2993`, `court.py:476`, `pipe.py:445`, `check.py:1117`, `lib/town/kit.py:485-531`, and the SDK README's own example | `step.ground(x, y)`: plumbing only, which "the SDK computes nothing" allows | Yes |
| Reckons a floor's level from the ground | `posts_in` and `inside` (`lay.py:597, 791`) copied into `court.py:440, 452` so that "a court's level and the ground its halls then read are one number" | An engine read: a field's mean over a region's posts, with no domain noun (R23). Medium | Its terraces |
| Finds what it produced for whom | `check.py:1001` copies `lay.py:2927`'s `key_of`; `check.py:1023, 1029` tie a storey and a plot to their building by rebuilding lay's key; eight `measure.py` parse `lay#<building>_s<k>` | `lay` writes `of = <building>` on what it returns (a kinds.toml attribute), or the core adds the call's `for_each_item` beside `by` | No |
| Says each code's `explain` once | `said` sets at `check.py:1173`, `row.py:426` | `step.explain(EXPLAIN)` once a call | Yes |

Twenty `_list` and six `_json` definitions in `lib/` go with the first two rows. No library folds
records (the engine does, `data.count`); and mapping an asset to its model's frame is the pack's
`front`, whose gap is the measured plan of section 1, not the SDK's.

## 5. Challenge 33

It does not close with K10. Its rows ask for market's change scenarios (a piece remodelled, a
module changed, the street moved) on a real kit. K10 lays a real kit through lib/building and
runs one of the three on it (`revised`, the window re-exported 1.1 times long). Not shown on a
real kit: a street moved (borrowed's row is never moved) and anything through lib/town, whose kit
run takes the first id per role (`kit.py:10, 146`), reads no `shared`, and would lay Kenney's
sample houses at a quarter of a person's size. Market's own debt stands: rows on unlevelled
ground, 85 of 189 doors within 0.3 m of the footway.

What would close it: first a ruling on whether lib/town's kit run is still the town's way, now
that `row.py` lays rows of kit-built houses and `realm` moved a village of them (833 instances
followed). If it is, market on Kenney's sample houses after a scale factor, with lib/town
reading `shared` (section 4, first row). If not, narrow lib/town's run to whole-house pieces and
close 33 on `borrowed` with a `move_street` case beside `revised`. Separately, row 32 in the
"Owed" table still says "Connectors and modules are M5", though `shared.module` (K6) and the
joint (K9) show both, and says mounts are measured from the model, which they are not.

## 6. What is owed, ranked

1. **The storey's height and the plinth from the kit** (section 2): 28 of 318 numbers, filed
   since K6. Small.
2. **One kit reader, attribute folding and ground reads in `viasdk`** (section 4): six, five and
   six copies, and the precondition for lib/town reading `shared`. Small.
3. **The ruling on challenge 33**, then `borrowed`'s `move_street`. Small; market on Kenney is
   medium and waits on 6.
4. **Pitch and span from the model** in every reader, restatements dropped from the packs: 80 of
   292 numbers. Small.
5. **A measured plan from geometry**, so a quarter-turned corner is said and L footprints are not
   typed. Medium, core.
6. **A scale factor in the pack** (`spec/assets.md`, filed): the one thing that keeps Kenney out
   of a person's world. Medium.
7. **Records**: `kenney.toml`'s dead `bay = true`; the works asset count (39 for 23); the stale
   paragraph in `kit-contract.md`, "The kit as a whole", which still says the taxon "does not
   yet" carry the kit's facts; challenge 32's row; one protocol for "written". Small.
8. **Mounts from model nodes** (51 of works' 85 numbers). Medium.
9. **A kit a second world can name**, replacing realm's 115-line merge. Large, core.
