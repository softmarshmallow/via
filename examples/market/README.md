# market

A market street on a slope, built from a box kit (docs/plan.md, M5; TODO "Dogfood: the
market-street district"; challenge 33). Read from the spike's market world for what it showed:
streets first, rows of houses that stand along a street and follow it, a kit of one module.

| File | What |
|---|---|
| `world.toml` | The land, four streets (lib/town `main_street`, `street`), the square (a `terrace`), ten row lines that `follow` their streets, two kit steps (lib/town/kit.py), the paint, eight views. |
| `packs/kit.toml`, `packs/kit/` | The box kit: `row_bay` (5 m, `role:wall`), `row_filler` (2.5 m), `row_end` (6 m, hipped), `row_corner` (6 m square, three storeys, a side door); 8 m deep, a 0.6 m plinth (`sink`). Made by `packs/make_kit.py`; facts in `kit.facts.toml`. |
| `packs/remodel.toml`, `packs/module.toml` | The kit with one piece revised; their facts files are the kit's, accepted before. |
| `changes/` | `remodel`, `module`, `move_street` (the three changes), `move_bend` (the street moved another way). |
| `expect.toml`, `measure.py` | The counts and the three changes, measured, with the protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python
python3 examples/market/packs/make_kit.py                        # only after editing it
via trust examples/market/world.toml --yes                         # and once per --change
via build examples/market/world.toml --out build/market --store build/market.store
via build examples/market/world.toml --change examples/market/changes/module.toml \
    --against build/market --out build/market+module --store build/market.store
python3.11 examples/market/measure.py build/market+module --base build/market --check module
engines/godot/sheet.sh --before build/market --after build/market+module --out review/market_module
```

`move_street` and `move_bend` are built on a store of their own (`--store build/market+move.store`):
on the base's store they stop with `diag.internal` (TODO, "From M5 slice 3, market"). No
`--option paths=…`: the world paints its own streets.

## How the rows are made

Each row is a line `offset` metres from its street's centre line, on the side the row stands:
the street's verge (`keep_out` to 5.75 m on the main street, 4.25 m on a street) and half a
house's depth out, so each front is on the footway's back edge. The line `follows` its street
over a `span`, so it moves when the street moves. The kit run (`lib/town/kit.py`, `caps =
"inside"`) lays on it: an end house at each free end and either side of a gap where a cross
street's verge reaches within half a bay's depth of the line; a corner house where the line turns
10 degrees or more; bays between, fillers for what bays leave, the rest to the free end; where
two neighbours' chords turn, or would meet once rounded to the millimetre, the joint opens
(rule 7), and a stretch that no longer holds its pieces gives one up.

## The three changes (base: 189 pieces, 0 errors, 0 warnings)

- **A piece remodelled** (`remodel`): the end house a metre deeper. `asset.revised` names all 24
  end houses, exactly the build's instances of `row_end`; each now stands half a metre over its
  footway (24 `place.keep_out`). No piece is added or removed; 10 bays beside an end on a bend move,
  since rule 7 reads the deeper plan.
- **A piece's module changed** (`module`): the bay 6 m in place of 5 m. Every row re-fits: 157
  bays and 6 fillers become 131 and 4, 189 pieces 161, every stretch still closes, no neighbours
  meet; `asset.revised` names all 131 bays. 28 names past each row's new last piece go.
- **The street moved** (`move_street`): the main street's entry swings 9 m south about the first
  bend, its length kept. Its two entry rows follow (28 pieces moved, 9 added, 7 removed; the first
  bend now turns 5.6 degrees, so its corner house goes); the other streets' 102 pieces keep their
  names and places.

`move_bend` moves the entry and the first bend 8 m south, so the street is 3.5 m longer. The
cross streets' feet are placed on it by `t`, a fraction of its length, so they slide 1 to 3 m, and
101 of the other streets' 102 pieces move with them; the church street's moved foot now meets the
main street off its line (`geom.unowned_crossing`). Placing a foot by `t` says "a fraction along";
a foot that should stay put needs a place that does not scale with the length.

## Views

`arrive`, `top` (along the main street from each end), `corner_low`, `corner_bend` (the corner
houses inside the first and third bends), `mill_ends`, `church_ends` (the end houses either side
of a cross street's mouth), `square`, `over`.
