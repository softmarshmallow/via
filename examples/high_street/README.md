# examples/high_street: a market street of someone else's kit, at a person's size

T1 of `docs/plan-fixes.md`, and the close of challenge 33 as far as it goes (R79): a town street
built the way a studio builds one, from a modular kit nobody here made, with the town's three
changes run on it. The kit is **Kenney's Modular Buildings 2.1**, by Kenney (www.kenney.nl), under
**CC0 1.0** (credit is not required, and is given here), fetched by
`examples/borrowed/fetch.py` and never committed.

```bash
python3 examples/borrowed/fetch.py                          # the models, into examples/borrowed/packs/kenney/
via pack accept examples/high_street/packs/kenney.toml      # after the pack changes
python3 examples/high_street/packs/make_cases.py            # then the change packs from it
via build examples/high_street/world.toml --out build/hs --store build/hs.store
via build examples/high_street/world.toml --change examples/high_street/changes/move_street.toml \
    --against build/hs --out build/hs+move --store build/hs.store
engines/godot/sheet.sh --before build/hs --after build/hs+move --out review/hs --pack $PWD/examples/borrowed/packs/kenney
```

| File | What it is |
|---|---|
| `world.toml` | The land, the high street and the church lane (lib/town), five frontage lines that `follow` them, the ground's paint, three views on the streets |
| `layers/houses.toml` | Five rows of houses, one along each frontage (lib/building's `row`) |
| `layers/walls.toml` | `lay` and `check` |
| `packs/kenney.toml` | Kenney's kit at `scale = 4`, its stated lengths in metres; `packs/make_cases.py` writes the change packs from it |
| `changes/` | A piece remodelled, the module changed, the street moved |
| `measure.py`, `expect.toml` | The measures, and the rows a build must hold |

Sheet: `docs/reports/high_street-sheet.png` (the base beside `move_street`).

## What it shows

**The street is the one thing written where it is.** A row of houses had a frontage of two
numbers (`at`, `to`), so a moved street left its rows behind: `examples/realm`'s `village_moved`
wrote ten numbers again. Here each side of each street is a line that `follows` it at the back of
its verge (geometry.md 4), and a row runs `along` that line (row.py rule 10; `reverse` for the
street's right side, since a line runs the way its street does). The row writes no lots: it
fills its line with houses one, two or three blocks wide (`widths`, rule 11), picked by the row's
name, so the choice is the same every build, neighbours differ, and a longer street gets more
houses while the ones before keep their names. Heights come from a list the same way (rule 5).
So the whole street is 19 tables and 78 numbers, and 49 houses and 2320 pieces are laid from it.
A row's own point stands on its line's start (`on`); a row whose line now bends or holds no lot
is not laid and is said (`row_bent`, `row_short`), and the other rows are laid.

**Kenney's kit at a person's size.** The kit is made at a quarter of a person's size (a doorway
0.25 by 0.5: `examples/borrowed`, "Scale"). The pack reads it at `scale = 4` (spec/assets.md 2.4,
the fix plan's F8): a block 4 m, a storey 2.5 m, a doorway 1 m by 2 m. What a kit states is in
metres, so its `shared` lengths are stated again at this size: the pack is borrowed's with five
lengths and the scale changed, not an include of it (spec/assets.md 2.8 would prefix its ids and
read its lengths as written). That costs one pack of 104 lines.

**On a slope, level.** The land rises 1 in 33 to the north. Each house stands on its own level plot
(`ground = "level"`, lay.py rule 19), passed on by its row (row.py rule 8). Two houses side by
side had each levelled 1.5 m past their shared wall, so the later by name took the other's ground
(`plot_lost` on every house of the lane). Three mends, each in the library: a plot keeps off a side
its building calls `party`; a plot's own ground is its level after every plot's bank (`kind.plot`'s
`flat`, order 2, so the sill is order 3); and `check` reads a party wall from the outlines, not from
the word, and reads no ground there or within one post of its ends, where the ground, a field of
posts, steps from one house's level to the next. So the lane's terrace steps with its land at its
party walls, and every post inside every house is its own plot's level (read from the height field:
within 0.000001 m). No warning in the base.

## The changes (challenge 33)

| Change | What is edited | What the build does |
|---|---|---|
| `remodel` | The `windows` block swapped for Kenney's large window (same box) | `asset.remodelled` (triangles 44 to 28); no piece added, removed or moved; every house kept |
| `module` | The kit read at `scale = 5`, a block 5 m | `asset.revised` names nine assets; every row re-fills the same frontages, 49 houses to 38; two lots west of the lane not returned (`row_meets`): a house 25 m deep reaches the lane's first, whose row starts at a share of the lane |
| `move_street` | The high street 12 m south, the lane's foot 32 m east: six numbers | The lines, the rows and every house follow; the lane is longer and holds 14 houses for 12; 52 houses, every door on its street, none in another's place. The views stand on the streets (`on`), so they follow too: written as numbers, the moved street's eyes stood inside its houses |

## What is owed

- **A street off the axes.** `lay` tiles no flat roof, and no floor, on a building turned off the
  axes: two tiles that abut there overlap by the millimetre's rounding, which the core takes for
  two things in one place (lay.py rule 9; rules.md 8.1). So both streets run along the axes.
  First built at an angle: no house had a roof (`roof_none`).
- **A row's frontage in shares.** A `follows` line's `span` is a share of its street, so where a
  row should start past another row's houses, it starts at a share that held for the houses of
  the day: the module change's deeper houses meet the lane's first.
- **A stair for a kit with none.** Kenney's blocks are closed and the kit has no stair, but `lay`
  asks a stair of every building of two storeys (lay.py rule 3), so the pack states one. It lays
  no piece; it is a stairwell no one sees.
- **The ground's paint.** The footways are painted `flags` to 5.25 m and the houses stand at 6 m,
  so a strip of grass shows before each front; the renderer's blend at 1 m posts widens it.
- **What a row leaves, when a lot is refused.** `left` is what a row's widths leave at its line's
  end; a lot `row_meets` refuses is not counted in it (`module`: `row_wn` leaves 0.4 m, and two of its
  lots, about 45 m, are refused).
