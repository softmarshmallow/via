# The canonical demo (D of docs/plan-fixes.md)

Asked by the owner on 2026-11-24, the plan's final milestone: one world that both proves the tech
and is visually astonishing, so that a person who sees it once knows why to use via. Design note,
written before code, from what F1 to T1 left standing; where what was built differs, this note
says what was built and why (marked **Built:**).

## What one look has to say

via's promise, the first line of `README.md` (`docs/doctrine.md`): decide what matters; the world
settles around it. Change your mind, and it settles again. You never repair a map by hand. A
person looking for a few seconds sees pictures, not prose, so the demo says that promise as a
picture:

> **One line changes. The whole town follows.**

A town in a wooded valley, rendered well. A line of its text changes: the lane's foot moves along
the high street. Rebuilt, the houses re-plan along the moved lane, the woods give way where houses
now stand and grow back where they no longer do, the ground levels under every new house, every
door still opens on its street. **Built:** the line is the high street's path, moved 40 m north
(y 125 to 165): moving the lane's foot alone turned the lane off the axes, and lay roofs no turned
house (42 crowded pieces, a house with no roof). The woods follow the street (their cover reads its
distance) as well as keeping off every house's plot. Nobody touched a house or a tree. The proof is in the same frame:
the line that changed, and the build's own numbers.

## The three pieces

1. **The hero still** (`docs/reports/demo-hero.png`): the town at golden hour from a hill above it,
   large. Beneath it, three panels: the text diff (one number), the same view after the rebuild,
   and a strip of numbers read from the two builds (**Built:** the before and the after side by
   side from one fixed eye over the stream's far bank, the diff and the numbers beneath) (houses re-laid, trees moved, overlaps 0,
   doors on their street, steps re-run of the total, seconds). Every number is a measure of
   `examples/demo/measure.py` with its protocol in `expect.toml`, as every example's is.
2. **The sweep** (`docs/reports/demo-sweep.mp4`, and a GIF for where video does not play): the
   lane's foot swept along the high street in even steps (**Built:** the high street's path, y 125
   to 165 in steps of 5), each step a real incremental build on
   one store, each rendered from the same camera. Played as frames, the town reflows as the one
   number moves, with the number and the build's time on each frame. Not an animation made by
   hand or by interpolation: every frame is a build anyone can run again.
3. **The world itself** (`examples/demo`), a normal example: its README, its changes, its rows.
   What the demo claims is what its `expect.toml` holds.

## The knobs (asked by the owner on 2026-11-25)

"Not only simply offsetting the path": the owner asked that the demo turn as many of the world's
knobs as it can, within the same base, so that a person sees at once a procedural framework, and
that the camera orbit while the map changes, as a timelapse.

- **One line a knob.** Each knob is one line of the world's text, in a change file of its own
  (`changes/`): the land's swell and the woods' cover (two `[let]` names the formulas read), the
  stream's width, the high street's path, the town's height and grain (the rows step's
  `params`, one line for every row: row.py rule 12, new), what the woods are (the scatter's
  `assets`), the seed. The town's style was written on each of five rows; rule 12 lets the rows
  step give every row its style where the row writes none, so one line sets a town.
- **The gallery** (`docs/reports/demo-knobs.png`): the base and each knob turned alone, from one
  eye, each lettered with its line and its numbers.
- **The timelapse** (`docs/reports/demo-timelapse.mp4`): the knobs turned one after another,
  each state the base and the knobs so far (a `--change` each), built on one store; the camera on
  one orbit round the valley's south (engines/godot/showcase.gd `--orbit`), each state on its own
  arc and blended into the next over the frames they share, so the camera never stops and the map
  changes under it.

## The world

- **Kept from T1**: `examples/high_street`'s street of Kenney's kit at a person's size, rows that
  follow their streets and fill them (row.py rules 10, 11), level plots that step at party walls.
- **Added around it**: a valley (**Built:** an expression of the height field, not a lib/land
  step: one build of the land a minute is the budget), a stream along its floor
  (lib/water) that the high street crosses or runs beside, and woods and underbrush scattered by
  cover (core scatters, `examples/vale`'s form) that give way to the streets' verges and the
  houses' footprints.
- **One pack that includes two kits** (spec/assets.md 2.8, F6): the town's Kenney pack and the
  shipped starter pack's trees and bushes, each under its prefix. The demo is then also F6's
  first use outside realm.
- **Streets along the axes**: lay tiles no flat roof on a building turned off them (TODO,
  lib/building). The sweep moves the high street along an axis, so every house stays square.

## The look

The sheet tool's rig is a measuring instrument (engines/godot/rig.gd: a linear tone map, a fixed
neutral ambient, the sky not reflected, the Compatibility renderer), so that a colour drawn is a
colour declared. The demo is a picture to be looked at, so it gets its own rig and its own
renderer, kept apart from the sheet's: `engines/godot/showcase.gd`, Forward+, a low warm sun with
soft shadows, the sky's light and reflections, a filmic tone map, ambient occlusion, a little
aerial fog. It loads builds through the same loader (`addons/via_loader`), so what it draws is
the build. It writes PNG frames at a fixed size; ffmpeg joins them.

## What it proves, and how each claim is held

| Claim | Held by |
|---|---|
| One line changed | `changes/north.toml`: one table, one key; the hero shows its diff against the world as written |
| Nothing was placed by hand | `hand_placed`, 0: no `[instance]`, no `[pin]`, no feature of kind `building` in the world or its layers |
| The world settled | Houses re-laid, trees added and removed, by name against the base (`added`, `removed`, `moved`) |
| It settled correctly | 0 overlaps, every door on its street, 0 errors, warnings named |
| What it cost | Build nodes run again of the total, and the build's time, from the run record, the change built straight after the base (a time is the machine's: its band is wide). Not "fast": moving the high street moves almost everything |
| The same text gives the same world | A second build of the base on a fresh store: the same build digest (checked twice, by the build and by the review; README.md) |

## Not in the demo, said

- A street at an angle, a river crossed by a bridge of the kit, people or traffic: each owed or
  not ours. The demo shows what holds today, and its README says what does not.
- The look is Godot's with our rig: no asset is made for the demo. Kenney's kit and the starter
  pack's trees are what a person starting today would have.

## Order of work

1. `examples/demo`: the world, its pack with two includes, its change (the high street moved),
   `measure.py`, `expect.toml`, README.
2. `engines/godot/showcase.gd`: the rig, a camera from a bookmark, frames to PNG.
3. The sweep: a script that writes N change files, builds each on one store, renders each frame,
   and joins them; its numbers kept with its protocol.
4. The hero still, composed from two frames, the diff and the numbers.
5. One independent review, one fix pass, the record.
