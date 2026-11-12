# Challenges: every request, and how it fared

This is the list the rewrite must answer to. The spike's code is thrown away; the requests are not.
Each row is something that was asked for, whether it fitted the model or not, with where it was
demonstrated, how well it held, and what it strains. A request that does not fit is recorded, not
skipped. Choosing to leave one out of the product is allowed, but it is a decision to write down
here, with the reason.

Rules for this file:

- Every capability that is asked for gets a demonstration in the spike, however crude, and a row.
- "Held" means it works in a world you can compile and look at. "Partly" names what does not.
  "Not shown" means there is only talk or a proposal, which is a debt.
- "Tier" is a first guess at who would own it: **core** (the engine), **library** (a domain
  library of ours, imported by choice), **user** (their content or code), **loader** (engine side).

Worlds live in `spike/worlds/`. Evidence and lessons are in `spike/FINDINGS.md`.

## Owed: asked for and not yet shown

| # | Request | Why it is owed |
|---|---|---|
| 11 | Sky, clouds and distant landscape | Discussed as a boundary question. Nothing built beyond a `look` table. |
| 16 | Warn when a large object has no collision | An idea for a lint. Not built. |
| 30 | A step that uses outside data (weather, a spreadsheet) | Claimed possible by construction. No step reads anything external. |
| 32 | The asset contract: front, pivot, module, connectors | Studied and proposed. The pack is unchanged. |
| 33 | The town's change scenarios | The town is built. A revised kit piece and a moved street were never run. |
| 41 | Unreal | Named as a target from the first message. Nothing exists. |
| 42 | Streaming and per-chunk loading | Budgets per chunk exist. Nothing loads or unloads. |

## The list

| # | Request | Shown in | Held | Tier | What it strains |
|---|---|---|---|---|---|
| 1 | A world from text; seams resolved from intent, no magic numbers | `vale` | Held | core | Expressions run as Python; needs a defined language |
| 2 | Real assets, swappable, never named in the world | `vale` with two packs | Held | core | A build lists the whole pack, not what it uses |
| 3 | Walk it in the engine | viewer, every world | Held | loader | The viewer is one world's worth of special cases |
| 4 | At least 30 frames a second | `vale` (114), `coast` (22 after parts) | Partly | loader, with core handing over batches | No frame-rate promise is ours, but the hand-off shape is |
| 5 | A lake that is not a boring shape: fill the ground and see | `vale` tarn | Held | core | Two floods that read each other silently miss an effect |
| 6 | A landmark versus scenery: which one gives way | `vale` mill pond and tarn | Held | core | Authority has three levels; volumes and decks ignore it |
| 7 | A cave and a sea cliff | `coast` | Partly | core | One overhang per column; the cave's inside was never checked |
| 8 | A cave that looks less artificial | `coast` | Partly | user | Judged by eye only |
| 9 | Recreate two reference photos in one map | `coast` | Partly | user | Matching a photo has no measure |
| 10 | Work from a concept or reference image | `city`, rebuilt after a reference map was shown | Partly | user, with core checks | Principles were read off the image by eye; nothing measures the match |
| 11 | Sky, clouds and distant landscape | none | Not shown | loader | Where our responsibility stops |
| 12 | Merge maps built apart into one | `coast` with `college` and `station` | Partly | core | Parts only translate; a floor shows terrain through it |
| 13 | A reach check: opt-in, a stated player, an escape hatch, user rules | `college`, `coast` | Partly | library, on core lints | Sees the top surface only; no decks, caves or interiors |
| 14 | An interior: levels, rooms, stairs, down to the props | `station` | Partly | library | Its vocabulary is written into the compiler |
| 15 | Walk indoors with collision and stairs | `station` walk test | Held | loader | |
| 16 | Warn when a large object has no collision | none | Not shown | library | |
| 17 | A kept list of references | `docs/references.md` | Held | | Links unverified |
| 18 | Ten times more dressing | `station` | Held | library | Sets, runs, dress and flank overlap in purpose |
| 19 | Hero and interactive objects: metadata, or define and query | `station` `interact` attributes | Partly | core pass-through | Only the pass-through pattern exists; kind attributes leak into it |
| 20 | A new campus from existing assets | `college` | Held | user | Kit bays never measured against each other |
| 21 | Layers: several owners, one world | `college` | Held | core | A list is replaced whole on merge |
| 22 | Parts: a true part and a place joined on | `coast` seam and portal | Partly | core | Two patterns under one name; clashes with "building part" |
| 23 | Walking must not teleport | viewer | Held | loader | Found late because the cause was a key binding |
| 24 | A player map, drawn from data, styled by an image model | `coast` map, in the viewer | Partly | core for data, user for style | No trustworthy measure of drift |
| 25 | Treat the spike as the product and clean it | `SPEC.md`, `check.py` | Held | | Section 12 is appended, not merged |
| 26 | Decide the language and setup | design page | Not final | | Rust core is a direction only |
| 27 | TOML, YAML or a third | measured excerpt | Held | core | Strings hide expressions; no null; no shared constants |
| 28 | The user's own code, in Python | `massif`, `city` lots | Partly | core lifecycle, user code | Only at layer boundaries; nothing inside a layer |
| 29 | Do not rerun what has not changed | `massif` | Partly | core | Covers inputs, not outputs; hidden inputs go stale |
| 30 | Any use: fetch weather and so on | none | Not shown | user | Reproducibility and trust |
| 31 | Pictures as inputs: a heightmap, noise, a mask | `relief` | Partly | core | No float formats, no tiles, no pass-through as a texture |
| 32 | The asset contract: where the front is, building blocks | study only | Not shown | core | The layer nobody standardises |
| 33 | A town the way a studio builds one | `market` | Partly | library and user | Rows are flat; no corners or fillers; change scenarios not run |
| 34 | A road network with bridges and an overpass | `city` | Partly | library on a core surface | No junction geometry; a way cannot be interrupted by a deck |
| 35 | A larger scale, like part of an open-world map | `city`, 1.6 km | Partly | core | 70 s at 1.25 m posts; tens of kilometres needs tiling |
| 36 | Sidewalk, roadway, railway; a waterway on the map only | `city`, `kinds/ways.toml` | Held | user vocabulary, core order | Effect order had to become explicit |
| 37 | Keep the spec and design current | `SPEC.md` 12, `docs/design/` | Held | | |
| 38 | A loop ramp | `city`, two interchanges | Partly | library on a core surface | Its end rests on another deck; no merge, no embankment, no banking |
| 39 | Who owns the load-bearing part; importable libraries | `city` road step | Partly | library | Road knowledge sits outside the compiler in a step; decks and kit runs are still inside it |
| 44 | Recreate an existing project's map exactly, linking its assets, owning only the master file | not in the repository | Set aside | core, a domain library and a converter | Set aside until after the first cut (doctrine section 7) |
| 45 | Own the asset contract, a taxonomy to manage assets by, and a zoo to browse and query them | `python -m worldc.zoo`, world `zoo_town`, `@taxon` in `example` | Partly | core mechanism, user vocabulary | Taxonomy, query and selection work in one place; the contract is still thin, nothing lints a pack alone, and usage is not tracked |
| 46 | Stress test: model a small mechanical object with full parts, as a modelling tool would | `sidearm`, and its `racked` variant | Partly | core | Parts as features and operations as kinds held with no compiler change; no frame inside a frame, no mutual constraint, one fixed order of operations |
| 47 | The same, from a real object's published numbers, with no hand-placed coordinates and optional accessories | `pistol`, its `racked` variant and three accessory overlays | Partly | core | Built size matches the published numbers exactly; needed formulas as attributes, planless kinds and a free solid; no rotation of a part, no mutual constraint |
| 48 | Doors that paths find, and ways that meet end to end, by name | `college` (three doors and their walks), `example` (lanes meeting the bridge) | Partly | core | A moved and turned building kept its walk; a moved bridge kept both lanes; a mount has no direction, height or type yet |
| 49 | Keep a hand edit when the rule under it changes; lock a finished area; see what a change did | `example` with pins, clears, a lock; `changes.json` | Partly | core | By place held 3 of 3 under six rule changes, by name fell to 0 of 3; points only, nothing for paths or painted fields |
| 50 | Placement that looks good unattended, with real assets | `glade`, base against `by_cause` | Partly | core mechanism, user rules | Clumps and growth by cause read far better than even scatter; judged by eye, no measure |
| 51 | An agent that did not write the tool builds a world from the documents alone | `monastery`, with its own log | Held | | One agent, one run; reachability and messages without a location cost the most |
| 52 | Two maps built apart, joined in a master that owns only the bridge between them; the rest blends by a strategy the user picks | `strait` (islands `a` and `b`), and its `move_b` overlay | Partly | core | The bridge names its ends in the parts and followed a 40 m move with no edit; the falloff is the user's expression. The blend knows distance from a rectangle, not what the ground is; parts do not rotate; nothing can be placed `on` a part's feature |
| 53 | One master map made of every world we have, joined automatically so it looks acceptable, to run and explore | `atlas`: nine worlds as seam parts and `station` on a door, with a creek, a bridge and eight roads owned by the master | Partly | core | Compiles clean in 86 s and walks; needed parts at any spacing, parts running their steps, a skirt outside the footprint, an explicit lift and renamed ground. Each world has its own zero, so the lift is found by eye; a high edge meets the plain as a straight bare scarp; a river or road stops at its world's edge; eight ground materials for the whole map |
| 54 | Two maps built apart that meet by ground, not a bridge: the developer attends only to the meeting (growth across it, the player funnelled through chosen openings), in a layer of its own | `border`: `college` and a new `wild`, joined by `layers/join.toml`; its `move_college` overlay | Partly | core mechanism, user rules | A ridge along the wild's edge, open where two walks cross, with expectations that prove every route passes an opening; all followed a 30 m move with no edit. Needed features laid after the parts, a part's edge as a path, an outline footprint and `closed` on an expectation. A side has a number not a name; a part's own growth stops at its edge; the master overrules a part's anchored ground with a warning only |
| 55 | A world's roads must not end out of nowhere at its edge: who owns the edge, the world or its parent | `city` (a ring street and nine declared exits), `atlas` (every exit answered), `atlas/changes/forgot_north_road.toml` (the failure) | Partly | core obligation, library kinds, user decisions | The city stops its own streets at a ring and declares what leaves; the atlas answers each exit by name with a road, a river or a `fade`; an unanswered exit is an error. The core reads an exit's `width`; an answer is not checked for class or width; a fade paints one material; nothing declares what a parent offers back |
| 56 | A city placed in a larger map must not read as a square; its road network should fade out as the larger map decides, more like an open-world city | `atlas` gives `city` a `city_limits` region and an `urban` field; `city` declares both with defaults and its two steps obey them | Partly | core mechanism (a world declares what it can be given), user rules (what a city does with it) | The outline and the thinning are now the parent's; the hand-drawn arterials are not, so they stay straight and stop at the limits; the parent cannot see the part's roads when it draws the limits; the waterfront cannot reach the sea; every change to what is given rebuilds the whole part |
| 57 | A city authored as intent at three strengths, as a text-first city tool would: anchored skeleton and landmarks, shaped districts with a character, everything finer derived; and when a landmark moves, the streets settle again | `atlas` declares two squares, four axes and five districts for `city`; overlay `move_market_square` | Partly | library (districts, patterns, lots) on core (`where` regions, a set handed down) | Three patterns and the change case held with no other edit. The main ways are still the city's own and hand-drawn, so the parent's districts cannot be said along them; streets, blocks and lots exist only inside two tools, so nothing can name, pin, clear or check one |
| 58 | Simulation under authority: state a few facts (a lake at a level, a river from A to B, high ground loosely) and let the existing Rust terrain model settle the rest; never generate many and pick one | `basin`, with overlays that move the mouth, state two ends only, and state what cannot hold | Partly | user step on a core step contract; the model is a library | Both facts held and followed a moved mouth with no other edit, in 3.5 s; a held cell proved right for a shore and wrong for a river; drainage is rectilinear at 12.5 m cells, tributaries carry no water, and the river is a straight ribbon |
| 59 | A number for "boxy", so it is not judged by eye alone | `kinds/tools/street_form.py`, run on `city`, the city in `atlas`, and `townland` | Held | library | Bearing spread 0.45 for the city alone, 0.76 in `atlas`, 0.87 in `townland`; for the main ways 0.40, 0.51, 0.97. It agrees with the eye. It says nothing about block shape or buildings, and has no reference from real towns yet |
| 60 | A city on simulated land whose skeleton follows it: main ways stated by their ends, the river as a stated fact, nothing drawn | `townland`, and its `move_tor` overlay | Partly | library on core places | Six main ways by ends, found over the land; the river goes round a stated hill; moving the hill re-settled river, crossings, ways and streets with no other edit. Built as its own world, not inside `atlas`. Tributaries are rectilinear, 166 warnings are buildings on sloping ground, and the search has no notion of a junction |
| 61 | Streets must not stop dead where they meet a river | `townland`: a `waterfront` along both banks (`embankment`), and the street tool joining or cutting back loose ends (`join_within`) | Held | library | Dead ends 39% to 15% (120 to 35 places; stubs under 40 m, 28 to 9). Streets that reach the water end on the embankment at a junction. The core has no "path along another feature", so the waterfront is written as a region; and a district written as a condition was silently cut in two by the river's banks until the condition named the river. Both are R13's missing mechanism. |
| 62 | A river must not read as a short, straight canal that begins from nothing | `townland`: the river enters at an `exit` on the map's edge, `width = [9, 17]`, `estuary`, `bends`; the water's surface carried under the banks | Partly | library | It comes in at the edge at full width, bends (sinuosity 1.05 to 1.22), widens to an estuary, and its shore is a smooth line, not 2 m steps. `exit` fits honestly: the parent must answer it as it answers a road. Bends are a sine with jitter, more regular than a real river; tributaries are still not drawn. |
| 63 | A bridge must be a bridge: straight, level, square to the river, with ground that meets it | `townland`: a `crossing` owns its deck and bridgeheads; main ways are led to the bridgeheads | Held | library | Before, the deck was cut out of the way that crossed, which doubled back at the crossing point; three decks lay on top of each other at one bridge. Now one straight deck per crossing, level within 0.6 m, 1.3 m clear of the water, ground raised to it. The step raises the ground by writing `height`; an abutment as a kind would be better. |
| 43 | A city whose roads make sense: hierarchy, districts, filled blocks | `city`, second version | Partly | user and library | The first version was valid and senseless; no check can tell them apart |
| 40 | Seasons, custom fields and spawn zones, points of interest | `vale` winter, pass-through kinds, `poi` | Held | core | `poi` and `bookmark` are names the compiler knows |
| 41 | Unreal | none | Not shown | loader | |
| 42 | Streaming and per-chunk loading | chunk budgets only | Not shown | loader, with core chunks | |

## What the list says about the rewrite

- **The core is asked for surfaces above and below the ground again and again** (7, 12, 13, 34,
  38): caves, interiors, decks, ramps. The spike answers each with a separate mechanism. One model
  of "more than one surface" is the largest unanswered design question.
- **Domain words keep landing in the compiler** (14, 18, 33, 34, 40). Every one of them worked,
  and every one is a noun the core should not know.
- **Change is asked about more than creation** (2, 21, 29, 33). The demonstrations mostly show
  building once.
- **"Partly" is the common result.** That is the useful finding: each names a specific thing the
  rewrite must decide to do or to leave out.
