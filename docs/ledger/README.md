# Ledger: the vocabulary of a game world

Every term someone has to know about when a world is built, from the big picture down to the
seams. The frame runs from a stylised exploration world, through a naturalistic wilderness, to a
dense modern city. A term being listed is not a promise to handle it. The point is that none of
them surprises us.

Each term is filed twice: under an archetype, which says what kind of thing it is, and under an
owner, which says whose it is to build. It is a dictionary, not a plan: a row marked "lib" is a
guess that the term belongs to a domain library, not a decision to write one.

## What is where

| File | Sections |
|---|---|
| [archetypes.md](archetypes.md) | The data types, the twenty archetypes with their contracts, and the three marks |
| [world.md](world.md) | 1 World structure and scale |
| [ground.md](ground.md) | 2 Terrain and ground; 3 Water; 4 Vegetation and ecology; 5 Land use and farming |
| [built.md](built.md) | 6 Ways; 7 Rail and transit; 8 Settlements and city structure; 9 Buildings and kits; 10 Asset facts |
| [play.md](play.md) | 11 Movement, traversal and collision; 12 Gameplay markup; 13 A living world; 17 Persistence and a live world |
| [delivery.md](delivery.md) | 14 Atmosphere, light and sound; 15 Seeing far and running fast; 16 The player's map; 18 Making it: pipeline and people |
| [seams.md](seams.md) | 19 Seams: where two things meet |
| [names.md](names.md) | 20 One thing, three names; 25 This project's own words: its nouns, its kinds and its steps |
| [looks.md](looks.md) | 21 to 24 Looks: the work, the theories, the tricks |
| This page | 26 What the ledger shows |

The section numbers are those of the single file this folder replaced on 2026-11-18
(`docs/ledger.md`), so "ledger section 19" still means the seams.

## The row

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|

- **Term:** the word a practitioner uses. Other names for the same thing share the cell.
- **Meaning:** one line.
- **Archetype:** one of the twenty in [archetypes.md](archetypes.md), Derivation and Network
  with their class; or one of the three marks (Runtime, Practice, Principle); or `-` for a row
  that fits none.
- **Whose:** a first guess at the owner. Marks may be combined; the first is the main owner.

  | Mark | Owner |
  |---|---|
  | core | the compiler: a mechanism with no domain word in it |
  | lib | a domain library of ours on top of the core |
  | pack | the asset contract: a fact stated about an asset |
  | loader | our engine-side loader for Godot or Unreal |
  | engine | the engine itself |
  | game | the user's game code and design |
  | carries | the game owns the meaning; the core carries the data to it |
  | ? | undecided |

- **Tool:** in sections 21 to 24 the fourth column is Tool, not Whose: what a tool could do at
  most. It is nothing, carry (hold the intent as data), rule (place by it), or measure (report a
  number, never a verdict).
- **Now:** where the first cut stands, the tree merged to `main` on 2026-11-18. It is `none`,
  or `shown:` or `partly:` followed by what shows it. `shown` means an example world, a shipped
  kind or a key of the format does this. `partly` means some of it exists. A row with nothing
  to point at is `none`. A pointer is one of:

  | Pointer | Names |
  |---|---|
  | `examples/vale`, `lib/review/reach.py` | a path in this repository |
  | `surfaces.md 1` | a section of a chapter in `spec/` |
  | `slope()` | a formula function (`spec/functions.toml`) |
  | `ways.road` | a kind of one of our libraries (`lib/ways/kinds.toml`) |
  | `clump`, `[scatter.<id>]` | a key or a table of the world format (`spec/vocabulary.md`) |

`python3 tools/ledger_check.py` checks every row against these rules: the archetype and the
marks come from their lists, every pointer names something that exists, no term is written
twice, and the counts below equal the rows. Run it with `--write` after adding rows.

## How it was made

1. One author's pass on 2026-11-12: 201 terms in 14 groups.
2. Four independent readers, each with one lens (stylised exploration, naturalistic wilderness,
   dense city, production in Godot and Unreal), returned about 400 additions and 40 corrections.
3. Four sections on looks were added: the work, the theories, and the tricks (sections 21 to 24).
   On 2026-11-17 their attributions and the engine names of section 20 were checked against
   sources, and a second reader's doubts were appended to section 24.
4. On 2026-11-13 a second ledger was read from sources in one working session: studio talks, a
   review of terrain modelling, level-design writing, one published map and screenshots of it.
   It filed 305 rows under fourteen archetypes.
5. On 2026-11-18 the two were merged here. The archetypes were tightened to twenty, each with a
   contract. Every row of both was filed under them. 114 rows of the second ledger were folded
   into rows the first already had, and 211 were added. Every `Now` was read again against the
   first cut and given a pointer; before that it recorded where an earlier prototype stood.

6. The same day the merge was read by others. A second reader filed a fifth of the rows (173,
   drawn section by section with a fixed seed) from [archetypes.md](archetypes.md) alone: 145
   were filed the same (84%; Cohen's kappa 0.83). Of the 28 that differed, 8 followed the second
   reader; the boundaries it found unclear are now written there under "Telling two apart", and
   8 more rows were refiled by them. Four other readers checked every `shown` and `partly`
   against the repository, 312 claims: 256 stood, 13 said too much, 15 too little, 27 pointed
   at the wrong thing and 1 had nothing behind it. All 56 were corrected. The sources cited
   were checked against their publishers: 60 of 65 stood as written and 5 were corrected.

Evidence is marked where it is stated. Each table's opening line says whether it is from general
knowledge or read from a source, and a row marked `(?)` is from memory. `docs/references.md`
holds the sources.

## How it is kept

1. A row is a term, a one-line meaning, one archetype, an owner and where the first cut stands.
   Add freely, one line each.
2. A `shown` or a `partly` names what shows it. A claim with nothing to point at is `none`.
3. A row that fits no archetype is filed `-` and listed below. Misfits are the signal that an
   archetype is missing or wrongly drawn.
4. After each new genre, or about fifty new rows, the ledger is consolidated: the check is run,
   an archetype that has become a catch-all is split, rows are re-filed, and the change is
   logged in [archetypes.md](archetypes.md).
5. A new genre is read as a table of substitutions over the same archetypes, as "A stylised
   reading" in [world.md](world.md) does.
6. A new archetype must meet the five criteria in [archetypes.md](archetypes.md).
7. The ledger records the domain. What we build about it is recorded where it is built: the
   `Now` column only points there.
8. A kind or a step of ours has a row in section 25 ([names.md](names.md)) with the archetypes
   it is made of. `python3 tools/layers_check.py` holds that section equal to `lib/`.

## 26. What the ledger shows

Counted by `tools/ledger_check.py`. "Ours" is a row whose first owner mark is core, lib, pack or
loader, or whose Tool mark is not "nothing".

<!-- counts: begin -->

865 term rows: 741 in sections 1 to 19, 124 in sections 21 to 24.

| Archetype | Rows | Ours | Ours, shown | Ours, partly | Ours, none |
|---|---|---|---|---|---|
| Field | 39 | 37 | 13 | 5 | 19 |
| Shape | 10 | 10 | 3 | 2 | 5 |
| Asset | 51 | 47 | 12 | 7 | 28 |
| Derivation | 49 | 47 | 10 | 9 | 28 |
| Simulation | 17 | 17 | 4 | 1 | 12 |
| Profile | 35 | 35 | 14 | 11 | 10 |
| Network | 24 | 23 | 8 | 3 | 12 |
| Area partition | 39 | 38 | 7 | 13 | 18 |
| Placement | 11 | 10 | 3 | 0 | 7 |
| Linear placement | 23 | 23 | 5 | 5 | 13 |
| Scatter | 41 | 41 | 10 | 13 | 18 |
| Recipe | 24 | 20 | 3 | 7 | 10 |
| Grammar | 18 | 18 | 2 | 4 | 12 |
| Representation switch | 18 | 18 | 5 | 2 | 11 |
| Meeting | 96 | 96 | 22 | 35 | 39 |
| Overlay | 15 | 14 | 5 | 3 | 6 |
| Guidance | 93 | 64 | 1 | 2 | 61 |
| Measure | 38 | 30 | 7 | 5 | 18 |
| Markup | 60 | 12 | 3 | 2 | 7 |
| Delivery | 40 | 34 | 5 | 6 | 23 |
| Runtime | 74 | 10 | 1 | 0 | 9 |
| Practice | 34 | 24 | 5 | 3 | 16 |
| Principle | 15 | 14 | 7 | 5 | 2 |
| - | 1 | 1 | 0 | 0 | 1 |
| **All** | **865** | **683** | **155** | **143** | **385** |

By class: Derivation (focal) 10, Derivation (global) 17, Derivation (local) 22, Network (directed) 10, Network (undirected) 14.

| Section | Rows | Ours | Ours, none |
|---|---|---|---|
| 1. World structure and scale | 83 | 64 | 31 |
| 2. Terrain and ground | 101 | 94 | 49 |
| 3. Water | 30 | 27 | 14 |
| 4. Vegetation and ecology | 64 | 61 | 30 |
| 5. Land use and farming | 6 | 6 | 1 |
| 6. Ways: roads, streets, paths | 79 | 78 | 39 |
| 7. Rail and transit | 10 | 9 | 9 |
| 8. Settlements and city structure | 22 | 22 | 11 |
| 9. Buildings and kits | 40 | 39 | 15 |
| 10. Asset facts | 30 | 30 | 23 |
| 11. Movement, traversal and collision | 27 | 8 | 4 |
| 12. Gameplay markup | 38 | 1 | 0 |
| 13. A living world | 31 | 1 | 1 |
| 14. Atmosphere, light and sound | 25 | 11 | 10 |
| 15. Seeing far and running fast | 22 | 14 | 10 |
| 16. The player's map | 11 | 9 | 3 |
| 17. Persistence and a live world | 8 | 5 | 2 |
| 18. Making it: pipeline and people | 28 | 24 | 2 |
| 19. Seams: where two things meet | 86 | 81 | 42 |
| 21. Looks: the work | 22 | 15 | 14 |
| 22. Looks: theories people rely on | 56 | 48 | 45 |
| 23. Looks: placement craft and tricks | 30 | 28 | 22 |
| 24. Looks: surface and finish tricks | 16 | 8 | 8 |

First owner mark, sections 1 to 19: core 169, lib 309, pack 71, loader 35, engine 36, game 118, carries 2, ? 1.
Tool mark, sections 21 to 24: nothing 25, carry 23, rule 38, measure 38.

<!-- counts: end -->

**Where the first cut stands.** The tables above. The rows that are ours and still none are
thickest in terrain and ground, the theories of looks, seams, and ways; by archetype in Guidance,
Meeting, Derivation, Asset and Delivery. The count before the first cut (2026-11-12, against the
prototype, 541 world terms) was 81 shown, 109 partly, 351 none; the two counts are not
comparable row for row, since rows were added and a claim now needs a pointer.

**What keeps recurring across groups.** These are the design questions the list points at, each
appearing under several headings, with where the first cut stands on each.

- *More than one surface.* Caves, interiors, decks, tunnels, stacked map layers, floating
  islands, basements, rooftops, underground concourses. There is now one model: a surface owned
  by a feature, with the ground cut beneath it (`spec/surfaces.md`), shown for a cave, floors
  and decks. Pits, floating land and tunnels are none.
- *Identity that survives.* Stable IDs, saves, overrides, hand edits, map reveal, quest hooks and
  live updates all rest on a thing keeping its name when the world is rebuilt. Names by place,
  pins and clears are shown (`spec/identity.md`); a lock is still refused. Saves and live
  updates are none.
- *Who gives way, and in what order.* Authority, sequence, blends between materials, biomes and
  districts, and every seam in section 19. Authority, order and owned meetings are stated
  (`spec/authority.md`). Meeting is still the archetype with the most rows that are ours and
  none.
- *Lanes, not roads.* Traffic, markings, signals, junctions, kerb cuts and parking all hang off
  lane-level data. Unchanged: every lane row of section 6 is none.
- *Causes, not placements.* Scree below a cliff, reeds by water, wear where feet go, dirt in
  corners: the wilderness reader and the looks sections ask for the same thing from two sides.
  Clumps and growth by cause are shown (`examples/glade`). A scattered thing's size and tint are
  still a number or a random range, and nothing aims or leans it by a field.
- *Movers, not a walker.* Reach, clearance and navigation differ for a person, a climber, a
  horse, a car and a glider. The reach check takes a stated body (`lib/review/reach.py`), one
  at a time, on foot.
- *An asset is a set with facts.* Files, simpler copies, collision, surface type, facing, pivot,
  module: none standardised, all needed, and now arriving uneven from generative tools. Facing,
  pivot, size and where pieces join are measured and kept (`spec/assets.md` 3 and 4). Collision
  layers, surface type and texture facts are none.

**What the looks sections add.** Of the terms about looks, a tool could do nothing for about a
fifth; the rest it could carry as intent, place by a rule or report as a measure (the Tool marks
above). The largest share that is ours to attempt is placement by rule: clustering, grounding,
growth and debris by cause, hiding a seam with a prop, breaking repetition. The measures
(sightlines, silhouette, spacing of sights, share of empty ground) can inform a person and
should never pass or fail a world.

**What to leave alone on purpose.** Rendering, weather, light, sound, crowds, traffic behaviour,
animal behaviour, combat, saves and co-op are the engine's or the game's: the rows marked
Runtime. For each, the most we do is carry the data that says where and what, in a form their
systems can read.

**Misfits.** One row is filed `-`: the volumetric extent limit (a full-volume model is costly,
so it covers only a small part of a world). It is about scale and belongs to no archetype.

## Depth of evidence

Read from sources in 2026-11: terrain method families, terrain analysis, geology and cliffs,
ground surface, terraforming, biomes and scatter, reveal and guidance, a stylised sky, and tool
mental models; one pass each on ambient life and road realisation. Everything else rests on
general knowledge. Only two styles of world have been read as pictures.

Subjects still to read from sources:

| Subject | What it would cover |
|---|---|
| Settlement siting and form | Why a town is where it is; village shapes; growth and historical layers; walls, squares, markets |
| Rural land | Field systems, hedgerows, terraces, orchards, farmsteads, tracks |
| Coast, ocean, world edge | Shorelines, beaches, islands, harbours; how a world ends |
| Climate, weather, seasons | Rain shadow, snow cover, wet and dry states, seasonal change |
| Crossings and other networks | Bridges, tunnels, fords, rail, canals, fortification lines |
| Interiors, dungeons, caves | Room graphs, floor plans, lock and key, cave generation |
| Gameplay layer | Traversal modes, navigation, encounters, resources, difficulty zoning, fast travel |
| Naming and metadata | Place names, addresses, signage, maps, regions and borders |
| History and wear | Ruins, ageing, set dressing, environmental storytelling |
| Plant and asset making | How a tree or a kit is itself generated; sockets, pivots, footprints |
| Validation | Reachability, soft locks, metric checks, playtest bots |
| Runtime and endless generation | Chunks, seeds, on-demand worlds, player changes |
| Other genres | Roguelike levels, city builders, voxel sandboxes, strategy maps, planets, racing tracks |

## Owed

- Two or three scenes per section: small situations that show its concepts together, each with
  one thing to move.
- Typical sizes and limits, each with its source.
- Links between rows: which concept feeds which.
- The operations an author performs (place, connect, flatten, enclose, reveal). The rows are
  nouns.
- No row is filed under Derivation (zonal) yet.
