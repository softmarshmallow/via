# Ways, kits and cities: what we own

Status: draft for discussion, 2026-11-12. It records what two spike worlds (`market`, `city`) and a
study (`reports/Modular city asset contracts.md`) support, what was decided on the way, and what is
still open. The compiler's behaviour is specified in `spike/SPEC.md` section 12; this page is the
reasoning. Nothing here is final.

## The question

A studio building a town or a city district works in a fixed order: streets, blocks, grey boxes,
kit buildings, dressing. It makes its kit by hand and assembles it with rules that live in a tool.
What it pays for is change: a street moves, a kit piece is revised, and hundreds of placements must
follow. Which parts of that should a text-first framework own, in a way that does not tie it to one
kind of city?

## Position

We own mechanisms and checks. The user owns vocabulary and content.

| Ours | The user's |
|---|---|
| A way is a path feature with a kind | Which ways exist: street, motorway, tram, canal towpath |
| A cross-section is a kind's effects, laid in a stated order | Widths, kerbs, materials, what a sidewalk is |
| A deck: a way that leaves the ground, with a clearance check | Where bridges go and what they pass over |
| A kit run: pieces of one module along a line or a frontage | The kit, its module, its pieces |
| The asset contract: the facts a pack states about a piece | Making the pieces to that contract |
| Diagnostics that name the piece and the reason | The rule for which building goes where |

`kinds/ways.toml` is a starting vocabulary shipped as an example. A project replaces it.

## Decisions taken in the spike

1. **Way types are kinds, not built-in.** Street, motorway, railway and waterway are entries in a
   library file. The compiler knows none of their names.
2. **Sidewalk and roadway are zones of one way, not separate features.** A street's kind lays a
   graded corridor, pavement, then roadway. One feature moves all of it.
3. **Effects carry an explicit `order`.** Without it, where two streets met, the second street's
   pavement lay across the first's roadway. Pavements are order 0, roadways 1, tracks 2. This
   answers, for one case, the open question of stating effect order instead of relying on the
   order of declaration.
4. **A waterway is a kind with a map style and no effects.** It appears on the player map and does
   nothing to the ground. "Map only" needed no new concept.
5. **A bridge and an overpass are the same thing: a deck.** The height field stays one surface. A
   deck is a second surface carried as a mesh, resting on the ground at both ends, with a profile
   between. It names what it passes `over` and the compiler measures the room it leaves.
6. **Rows stand `along` a street.** A kit run given a street, a side, a start and a number of bays
   follows that street when it moves.
7. **Where buildings go is user code.** The `city` world's 1,055 buildings come from a 100-line
   step that walks each frontage and keeps a lot where it is clear. It hands back 295 plain kit
   runs. The framework did not need to know what a lot or a block is.

8. **A ramp is a deck whose end rests on another deck.** The city's loop ramp leaves an avenue,
   turns three quarters of a circle and lands on the expressway; its end takes the overpass's
   height there and meets it within about 10 cm. Adding a grade limit to decks at the same time
   flagged two bridges that had been built too steep.

9. **A city is drawn at one level and derived below it.** The first `city` was a uniform grid
   with an expressway nothing joined. It compiled without a single error and made no sense, which
   no diagnostic could say. It was rebuilt as a hierarchy: 12 lines drawn by hand (a river, eight
   arterials, two freeways and two connectors), 23 district labels and 4 interchange points. A
   260-line road step derives the rest: 26 districts as the faces between arterials, 159 local
   streets on each district's own grid, 5 river bridges, 18 ramps including two loops. A second
   step fills the blocks with about 3,900 buildings. The compiler learnt no road knowledge.
10. **Blocks can be derived in user code.** The road step finds districts by polygonising the
    arterials, the river and the map's edge, and writes them back as region features that the lots
    step reads. The study's advice to make blocks a derived structure holds; it did not need to be
    the framework's.

## Proposed structure (not decided)

Three tiers. The **core** holds mechanisms with no domain nouns: fields and authority, frames and
relations, placement, a surface carried along a path, a region extruded between heights, the
lifecycle. A **domain library** of ours, imported by choice, holds the knowledge of one domain and
lowers to core entities: `ways` (cross-sections, decks, ramps, junctions), `building` (levels,
rooms, openings, kit runs), `city` (blocks, lots, frontage). The **user** holds content and their
own rules. The spike does not show this: it writes `kitrun`, `level`, `room` and `deck` into the
compiler. `docs/challenges.md` row 39 records it as owed.

## What the two worlds showed

- **`market`** (256 m): a hill town with four streets, a paved square and 15 rows, written by
  hand. Its first compile had three errors, all placement conflicts the compiler named with
  positions. The one real lesson: buildings that front a square must stand on the square's level,
  not each ask for ground of their own.
- **`city`, first version** (1,280 m): a uniform grid, an expressway on a bridge and an overpass,
  a railway, a ferry route. Valid and senseless. Replaced.
- **`city`** (1,600 m): the hierarchy described in decision 9. It compiles in 70 s, most of it
  spent compiling the lower layers again for each step. The clearance check caught the southern
  freeway passing 1.7 m over an arterial, which exposed that a deck could not hold its height.

## The asset contract (proposed, not built)

Today a pack states size, footprint, solidity, ground tolerance and one module length. "Front" is
an unwritten convention. The study found that almost nothing about a kit piece is standardised
anywhere, so this is the layer worth owning.

| Fact | Source |
|---|---|
| Up, front, unit | Adopt glTF's rule (+Y up, +Z front, metres); a per-asset correction when a model differs |
| Bounds, triangle count, lift | Measured, as now |
| Pivot | A rule declared once per kit; the model's origin is measured and checked against it |
| Module (run, depth, storey) | Declared, separate from bounds, with an allowed overhang |
| Role, family, variants | A kit table in the pack; never parsed from names |
| Connection points, surfaces, clearances | Declared; optionally read from named nodes and compared |
| Allowed stretch | Declared per piece, default none |

Plus a lock of these facts per pack revision, compared on every compile: a remodelled piece passes
silently, a changed module or pivot lists every instance it affects.

## One interface for anything with parts (proposed, 2026-11-12)

The pistol stress test and the asset contract turned out to be one question. A thing with parts,
whether a gun, a vehicle, a building kit piece or a village placed in a larger map, shows the same
four things to whatever uses it:

| It exposes | Pistol | Building kit piece | Vehicle | A placed map |
|---|---|---|---|---|
| facts: size, front, footprint | measured 21.9 x 3.3 x 14 | module, storey height | wheelbase, clearance | extent |
| mounts: named places to attach to | optic, muzzle, rail | wall ends, door opening | wheel hubs, seats, turret | where its roads leave, its door |
| states: the same thing in another condition | slide back | door open, damaged | doors open, wrecked | winter |
| options: things that may be added | sight, light | awning, sign | roof rack | an event's dressing |

The spike already has each of these under a different name at a different scale: a part's `seam`,
`portal` and `joins_at` are mounts; a variant is a state; an overlay is an option; a pack entry
holds facts. The proposal is to name them once and use them at every scale.

- **Global, not asset-specific.** Mount, state, option and facts are core vocabulary. An asset in
  a pack and a world placed as a part answer to the same interface.
- **The inside is free.** A thing may be a model file (its mounts read from named nodes or stated
  in the pack) or a document of ours (its mounts worked out by formula). Whoever uses it cannot
  tell and should not need to.
- **Not a modelling tool.** Making the inside stays the user's. We own what a thing shows to the
  outside, and the checks on it.

First step taken: mounts with a position exist in the spike, on assets and on path ends, and a
path can start or end at one (`spike/SPEC.md` 12.6c). What it still needs: a mount with a direction (which needs frames that nest and
turn), mounts with a type (so a wall end meets only a wall end), and checks that know which state
they are looking at.

## Open

- **Junctions.** Two ways crossing at grade each grade the ground in turn. Nothing models the
  junction: its corners, its kerb radii, who has the level. A level crossing is just two paints.
- **A deck's ends.** Nothing checks that a way meets a deck where it starts, or that the grades
  agree. The ways on each side are separate features that the author lines up.
- **Merges and embankments.** A ramp now lands on a deck, but nothing shapes the merge, and a
  ramp on fill is not expressible because a road follows the ground it finds.
- **Block, lot and frontage.** The study says to add them as derived structure. The spike got a
  city without them by leaving them to user code. Which is right is undecided; the cost of leaving
  them out is that no check can say "this building has no street".
- **Corners, ends, stretch and filler in a kit run.** Only weighted variants and one end piece
  exist. The study lists three industry answers to leftover length; none is built.
- **Tunnels.** A way under the ground needs the volume field, not a deck. Untried.
- **Sense.** Nothing in the framework can say a layout is implausible. Whether it should try, and
  with what (connectivity, hierarchy, dead ends), is open.
- **Scale.** The city is 1.3 km at 1 m posts. A real open-world map is tens of kilometres, which
  needs tiling the compiler does not have.
- **The naming clash** between the standards' "building part" and our "part".
