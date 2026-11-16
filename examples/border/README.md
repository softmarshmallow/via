# border

Two places built apart that meet by ground, not by a bridge (docs/plan.md M4; R8, R10,
composition.md 6; challenge 54). Rebuilt from the spike's `border` in the v1 vocabulary. The spike's
built place was a college, an M5 world; here it is `examples/monastery`, imported by file, unchanged.

| File | What |
|---|---|
| `world.toml` | Rank 0: the corner's bare ground (the agreed slope), `[import.wild]` (an L footprint around its kept corner, blended over 16 m from `wild.meet`), `[import.abbey]` (the monastery's shelf west of the gorge, a footprint shape in its own frame, `lift = 0`). |
| `layers/join.toml` | Rank 1, owner `join` (composition.md 6.5, R36): the two walks answering the wild's exits, the bank along the wild's two sides facing the abbey (`follows = wild.footprint`), open where each walk passes, the join step `fringe`, the fringe trees, six views, the reach lint. |
| `layers/steps/fringe.py` | A step at the join: reads `wild.cover` and `abbey.clearing`, returns the wild's cover carried out over the bare ground. |
| `lib/bank/` | This project's kind `bank`, shaped; `opens_for` names the walk whose crossing it owns. |
| `wild/` | The wild, built alone: hills, woods (a core scatter), two trails ending at two exits (lib/town `entry`). |
| `changes/` | `move_abbey` (30 m east; nothing in the join edited), `shut` (both openings shut, the wild stated sealed). |

## Build

```
export VIA_LIB=$PWD/lib
via build examples/border/wild/world.toml --out build/border_wild
via build examples/border/world.toml --out build/border
via build examples/border/world.toml --change examples/border/changes/move_abbey.toml --out build/border+move_abbey
python3.11 examples/border/measure.py build/border --alone wild=build/border_wild --check base
engines/godot/sheet.sh --before build/border --after build/border+move_abbey --out review/border
```

Cold build about 3 s (release, M4 slice 3 tree, one run).

## What the vocabulary could not say

- Growth across the join from the wild's own rule: the wild's scatter stops at its footprint, and
  nothing runs it over parent ground. The join step carries the wild's cover out; the trees on it
  are a second scatter that copies the wild's assets and spacing.
- The wild's sides have no names: the bank spans fractions of its outline (0.358333 to 0.5125,
  0.526667 to 0.6625). Two `follows` on one ring cross at its corner and cannot end on each other,
  so they stop 6 m apart with overlapping crests.
- A name attribute holds one name: one bank feature per opening walk.
- `blend` cannot name a place (6.3), so the distance to `wild.meet` is a parent field. (Since M4
  slice 4, 6.3 lets a blend read `dist` of an import's place by its scoped name; this field stays.)

## Known faults (2026-11-16, walked on the move_abbey sheet)

- (Fixed by M4 slice 4: a change built `--against` the base store builds, and equals a fresh store.)
- At the gate (`at_gate`) a sheer grey face stands beside the walk: not diagnosed.
- The monastery's views outside the footprint (bridge, deck, gorge, turns) are brought and look at
  bare grass: features come in whole even where their ground does not.
- (Fixed by M4 slice 4: paint is brought, composition.md 6.2; the monastery's generate steps, its
  tarn fill, run in the parent's build, composition.md 12. Its lints, the reach lint among them, do
  not run in the parent by design; the brought water is off the walks, `walk_wet` 0.)
- Two walks are blocked by ground: `at_gate` and `bank_corner` (TODO, From the M4 close).
