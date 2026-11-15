# townland

A river town of three districts on hilly land (docs/plan.md, M3; R15 to R18), the spike's
`townland` rebuilt in the v1 vocabulary on the new core. 512 m at 1 m posts. The author writes the
land, the river's course, one crossing, the waterfront, the places the main ways join, five main
ways by what they connect, and three districts; streets, blocks, lots and houses are derived.

| File | What |
|---|---|
| `world.toml` | The land (a valley written by its line, a hill `tor`), the river (lib/water `beck`), the crossing `old_bridge` (lib/ways `crossing`, new), the waterfront `embankment`, five routed main ways, three districts, three streets steps, one network per district, `[cells.lots]`. |
| `layers/town.toml` | Houses (lib/town), the lints (`review`, `review_houses`, `form`: lib/review/form.py, new; `reach`), the paint, eleven views at the meetings. |
| `changes/` | `move_hill` (fails, below), `move_crossing` (the bridge 23 m upstream), `edit_west` (the west end's west edge in 8 m). |
| `expect.toml`, `measure.py` | Counts, R13's names-elsewhere-unchanged, the street form and the decks, measured, with protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python
via trust examples/townland/world.toml --yes                       # and once per --change
via build examples/townland/world.toml --out build/townland
via build examples/townland/world.toml --change examples/townland/changes/move_crossing.toml \
    --against build/townland --out build/townland+move_crossing
python3.11 examples/townland/measure.py build/townland+move_crossing --base build/townland --check move_crossing
engines/godot/sheet.sh --before build/townland --after build/townland+move_crossing \
    --out review/townland --option water=tag:water_level
```

## What it shows (2026-11-15)

- R16: five main ways written by their ends, routed over the land; they arrive at the crossing's
  ends by name (`old_bridge.start`, `old_bridge.end`) and meet at the market.
- R18: one crossing, one deck, 0 doubled, 0 ways over the water (spike: 3 overlapping decks, then
  1); `move_crossing` moves the bridge whole and the ways follow it. The embankment owns the east
  bank and the old town's streets end on it. 0 `geom.unowned_crossing`.
- R15: lib/review/form.py reports bearing spread 0.86 (main ways 0.87), orientation order 0.29,
  four-way 12%, three-way 35%, dead ends 53% (18 of 34), each record with its reference and protocol.
- R13: `edit_west` changes no block, lot, street or house name on the east bank.
- The reach lint: a walker from the west gate reaches the market and the other gates over the
  crossing, and every door from the market (70 of 70).

## Known faults (read from the sheet, on foot)

- **The deck is a causeway.** Kind surfaces are `doc.not_yet`, so lib/ways' `crossing` writes its
  deck on the terrain (a stand-in, said in the kind): the water stops at both sides of it.
- **Dead ends 53%, worse than the spike's 15%.** 15 of the 18 are streets ending on a district's
  outline into open land; none inside the town. The districts are apart from one another; nothing
  owns the town's edge (an edge lane, or districts that meet along a main way).
- **It reads as fields with a few houses.** 66 houses on 39 of 99 lots; 11 streets. At 1 m posts
  the paint smears: ways are wide grey smudges with no kerb.
- **`move_hill` fails**: `formula.outside_field` on `north_road`'s bed, cause not found.
- `west_streets_view` stands against a house wall; `from_east_gate` sees no town. Re-aim them.
- The river is a written course and the valley is a formula of its line (`dist('river')` at every
  post, read by five route costs, took over ten minutes); a river found round the hill is not shown.
- A cold build is 106 s at 1 m posts; at 0.5 m it was 390 s.
