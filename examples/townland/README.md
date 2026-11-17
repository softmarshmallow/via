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

A cold build takes about 27 s at 1 m posts (release, fresh store, 3 runs, M3 slice 4; 93 s at
4d331d3 before the kernel's fast paths, TODO.md "Distance to large regions is slow").

## What it shows (2026-11-15)

- R16: five main ways written by their ends, routed over the land; they arrive at the crossing's
  ends by name (`old_bridge.start`, `old_bridge.end`) and meet at the market.
- R18: one crossing, one deck, 0 doubled, 0 ways over the water (spike: 3 overlapping decks, then
  1); `move_crossing` moves the bridge whole and the ways follow it. The embankment owns the east
  bank and one of the old town's streets ends on it. 0 `geom.unowned_crossing`.
- R15: lib/review/form.py reports the street form with its reference and protocol in each record;
  16 street ends on a district's outline are counted as edge ends, 0 dead ends inside the town.
- R13: `edit_west` changes no block, lot, street or house name on the east bank.
- The reach lint: a walker from the west gate reaches the market and the other gates over the
  crossing, and every door from the market (103 of 103; 101 of 101 since M5, over the deck through
  its meetings, surfaces.md 4.4).

## Known faults (read from the sheet, on foot; townland pass 2026-11-15 evening)

- **The deck is still a causeway**, though no longer under water. Kind surfaces are `doc.not_yet`,
  so lib/ways' `crossing` writes its deck on the terrain (a stand-in). Its new `water` effect holds
  the level under the deck at `deck_z - freeboard` (here 6.0 m, the river's level): 0 of 192 deck
  posts wet (was 56, up to 0.61 m), and the water meets the deck's two faces flat. lib/water's beck
  reads the floor where it cut it, so the deck is no longer smoothed into the river (water 3.1 m
  over the line at the deck, now 0.25 m at most beside it). Nothing passes under it yet.
- (Fixed, M3 slice 5 ways) **The embankment's ends were sheer** (65 to 69 degrees past them, a dry
  pit 0.19 m below the river at the south end) and it stopped 34 m short of `old_bridge.end`. It now
  starts at the crossing's end by name and rises to its deck (`level_start = "= old_bridge.deck_z"`),
  and its wall fades over each end (lib/town `waterfront`): 28.5 degrees past the south end, no pit.
  The old town gains that frontage: 144 lots, 95 houses (were 152, 99); on the combined slice 5
  tree 145 lots, 97 houses, 101 reach pairs held, 2 `place.relief`. One `authority.squeezed`
  warning remains at (202, 327), where the faded wall and the crossing's deck edge overlap at
  partial masks.
- (Fixed, M3 slice 5 ways) The river pooled 0.25 m against both faces of the deck (the beck's level
  smoothed from the stand-in deck): the crossing's `pass` holds it to the river's `flow`
  (`face_pool` 0.02). Ways reach the deck at 1:10 (`head_grade` 0.32 to 0.10).
- **One street of the old town ends on the embankment** (`old_streets#d68`; was none). The others
  that step 6 lays fall within `spacing / 2` of `quay_road` or `high_street`.
- **`from_east_gate` sees no town**: from the gate down the east road the hill's south shoulder
  hides it. A view from the east road's crest (t about 0.55) would.
- **move_hill empties the old town**: 52 of its houses go, `up_old_town` and `upper_view` look at
  grass. The district regions stay where the hill was.
- **4 `place.relief`** (1.6 m at most) on houses along the east road's steeper new course.
- The river is a written course and the valley is a formula of its line (`dist('river')` at every
  post, read by five route costs, took over ten minutes); a river found round the hill is not shown.
