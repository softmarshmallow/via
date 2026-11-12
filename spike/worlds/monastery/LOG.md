# LOG: building `monastery` from the documents alone

Read: docs/design/concepts.md, spike/README.md, spike/VOCABULARY.md (all), spike/SPEC.md sections 3.5-3.9,
3.17, 5.2, 6.1-6.4, 6.9, 12.2, 12.4-12.8 only, the example project, kinds/terrain.toml, review.toml,
building.toml, the head of ways.toml, the zoo listing of the pack, sdk/viasdk/__init__.py.
Not read: spike/worldc/, spike/viewer/. One compiler crash printed a traceback with four source lines
(entry 14); that is the only compiler source I saw.

Compile runs: 19 (16 of the world, variant and overlay; 3 probes with throwaway overlays). Renders: 1 run of shoot.sh.

## Rounds

1. First compile: 7 errors, 8 warnings. Fixed entries 1-4.
2. 1 error (`reach.not_sealed`), 6 warnings. Tilted the tarn floor, moved bookmarks, re-laid the road.
3. Still `reach.not_sealed`. Wrote my own probe over height.f32/water.f32 to find the route (entry 5);
   added a chockstone; raised the road's `grade`. 0 errors, 8 of 8 expectations.
4. Compiled the variant and the overlay, plus two throwaway overlays to test my lint and the mounts
   (entry 9, 10). Fixed `authority.squeezed` on the guesthouse; loosened my lint's gap (entry 8).
5. Rendered with shoot.sh, looked at five pictures. The hall (22 m) stood taller than the bell tower
   (20 m), so the tower is now scale 1.5. Final: base, winter, guesthouse all 0 errors, 1 warning each.

## Obstacles

1. **A field in a higher layer read by the world file, with a generate step between.**
   Wanted: `ground` (world.toml) painted by `f('forest')`, `forest` declared in layers/dress.toml as the
   example does. Tool said: `error field.unknown  below gardens.field.ground: f('forest') names something
   that is not declared: no field 'forest'`. The prefix `below gardens.` is explained in no section I read;
   I worked out that the step is handed a separate compile of the land layer plus the world file, where the
   dress layer does not exist, and that its diagnostics are reported too (so every land-layer diagnostic
   appears twice, once with the prefix). Moved `forest` and `grass` into world.toml.
   Ended: solved by guessing what the prefix means. Cost: one round.

2. **`reach.not_sealed` does not say where the route is.**
   Wanted: an expectation that the gorge cannot be crossed on the ground, so the bridge matters.
   Tool said: `error reach.not_sealed hall at (77, 190): expected out of reach ... but a route exists over
   the ground / hint: close the way in, or change the expectation`. `reach.cut_off` is documented to give
   the narrowest gap; `not_sealed` gives nothing, and reach.png is 321 px with no legend (SPEC 6.9 does not
   say what its colours mean). Two blind fixes (shorter taper, tilted tarn floor) did not close it. I then
   wrote a throwaway script that rebuilds the walkable mask from height.f32 and water.f32 by the rule in
   SPEC 6.9 and printed it as text: the route was the 4 m sill between the tarn's shore and the head of
   the gorge. Ended: worked round (own script, then entry 3). Cost: two rounds; the most expensive item.

3. **A stream that leaves a derived tarn cannot be sealed, and cannot be tied to it.**
   The flood sets its level `freeboard` below the spill, so the lip between tarn and stream is always dry
   and level: a ford. Cutting the gorge into the lip only lowers the spill. Also the stream's first vertex
   cannot be `on` the tarn (path vertices are typed; the flood itself "Needs `at`; fails with `on`",
   VOCABULARY section 1), so tarn `at`, `tarn_y` and the stream's first point are three typed numbers that
   must agree. Ended: worked round with a solid boulder (`chockstone`, `on = { feature = "stream", t = 0 }`)
   that blocks the lip for the reach lint. It is a trick: the lint counts explicit solid footprints.

4. **The brief's reach sentence cannot be written as one expectation.**
   "A walker can reach the main hall and the well from where the road enters the map" needs the bridge.
   Docs said so plainly (SPEC 6.9 "What it cannot see: ... bridges"; the example's dress.toml comment), so
   no time was lost. Wrote two ground legs (`road_entry` -> `bridge_foot_east`; `bridge_foot_west` -> hall,
   well, tower) and my own lint step `way` that checks each way of the chain starts where the last ends.
   Ended: worked round, from docs. The property is therefore asserted by three checks, none of which
   knows the deck's surface height or its parapets.

5. **`seam.change_exceeds` on the court and the garden plots.**
   `court ... terrace would move 'height' by 9.0 m; its limit is 8.0 m / hint: move it to gentler ground, or
   raise the limit on purpose`. Clear and correct. Court: raised `max_cut` to 10 on purpose. Gardens: 60 m
   strips had 2.9-3.6 m of change against my kind's 2.0; split each row in two in the generate step and set
   3.0. Ended: solved from the diagnostic.

6. **`seam.grade` on the switchbacks.** `road_up at (276.6, 69.9): road climbs at 23% after grading; its
   limit is 16% / hint: lengthen the route, raise grade, or accept a steeper limit`. Position and hint were
   right. Three tries (move vertices twice, then `grade = 45`). Switchbacks are five typed points; nothing
   lets me say "climb from A to B at 12%". Ended: solved from the diagnostic, by hand.

7. **The compound wall.** The pack's only wall (`wall.white`) is `solid = false` and 30 m long; nothing
   makes an enclosure with a gate. Used `[[run]] on = "court"` with `hedge` at scale 2 (6 m), started the
   court's outline at the gate jamb and chose 90 x 60 m so that every corner falls on a piece boundary
   (49 pieces, `to = 0.98`). The gate is "the part of the outline the run does not cover". Piece count and
   `to` are hand arithmetic from the outline length; see entry 14. Ended: worked round.

8. **Path ends in world.json are not the path's ends.** My lint compared `points[-1]` of one way with
   `points[0]` of the next. With `ends_at = "bridge.start"` the road's last written point was
   (218.08, 194.37) against the bridge's (217, 195): 1.3 m; `approach` ends at 147 where I typed 146.
   VOCABULARY 2.4 says of `path`: "Written out every eighth sample", but the SDK docstring and the example
   lint use `points` as the path. Ended: solved from docs after the fact; lint tolerance set to 2.5 m.

9. **Is my lint step really run and really failing?** A clean pass prints only `step way: lint ran`. I
   proved it with a throwaway overlay that tightens its params (`[[step]] id = "way"` + `[step.params]`),
   guessing that step rows merge by id like other lists. They do: `error way:broken bridge at (217, 195)`.
   Ended: solved by guessing.

10. **Do mounts follow a move?** Overlay moving only the bridge's `path`: `road_up`, `approach`,
    `court_walk`, lamps, bookmarks and the two reach guides all followed, 0 errors, 8 of 8. Worked first
    time from SPEC 12.6c. `ends_at = "hall.door"` (an asset mount) and a path with no `path` key at all
    (`court_walk`) also worked first time.

11. **`authority.squeezed` on the overlay's guesthouse.** `guesthouse at (149, 172): its apron meets the
    apron of 'court' with 3.9 m of 'height' between them and no free ground`. Clear. Moved it to the uphill
    side of the approach where the ground is at court level. Ended: solved from the diagnostic.

12. **`bookmark.blocked` and `bookmark.crowded`.** `blocked ... terrain hides the target 17 m from the
    eye / hint: raise eye, or move the bookmark`: helped, three bookmarks fixed in one pass.
    `crowded bm_gate ... the nearest is a 'lamp' 14 m ahead`: the lamps are the subject of that view, and
    they are not solid. Left as the one remaining warning; I found no key to accept it.

13. **Tint and scale.** `tint = [2.6, 1.6, 2.8]` on the wall run, to turn a green hedge stone grey: the
    render shows green walls. `explicit` rows in world.json carry neither `scale` nor `tint`, so I could
    not tell from the build whether either was applied. Removed the tint. Ended: gave up on colour.

14. **A formula in a run's `count`.** Wanted `count = "= 294 / wall_piece"`. SPEC 12.6b lists formulas for
    features, `[params]` and kind defaults only, so I tried it in a throwaway overlay to see what is said:
    a Python traceback (`ValueError: invalid literal for int() with base 10: '= 294 / wall_piece'`), no
    diagnostic. Ended: gave up; count typed.

15. **shoot.sh.** Pictures appeared within about a minute, the command returned only when its own
    240 s watchdog fired. Used once.

16. **The step cache.** `step gardens: generate ran` on almost every compile although only upper layers
    changed; `reused` appeared once, when the same build was compiled twice in a row. It looks as if
    gen/gardens/ is shared by base, variant and overlay builds, so alternating them always reruns. Not
    verified. Harmless at 0.2 s.

## Guesses (key names and behaviour)

- `f('height')` inside an effect on `water` (my gorge's thread of water): worked.
- `where(flow > 0, ..., 0)` with a scalar attribute in a mask: worked; `flow = 0` in winter leaves the bed dry
  (checked in water.f32).
- `[variant.winter.kind.gorge.defaults]`: a variant may override a kind default: worked.
- `face`, `scale` and a free key (`role`) on a `[[run]]` reach the children: `face` and `role` yes, `scale`
  not visible in the build.
- `[[expect]] to` takes a set member (`wellcourt.well`): worked.
- A bookmark `on` a deck path with an offset: worked.
- Regions carry `points` in world.json like paths: worked (my generate step depends on it).
- Deck `surface = "stone"`: the name is in the pack, not in VOCABULARY's list of surfaces; the build
  records `material: stone`.
- WRONG: a field declared in the top layer can be read by the world file's `ground` (entry 1).
- WRONG: a short taper, then a tilted floor, would seal the tarn's lip (entry 3).
- WRONG: tint above 1 would lighten an asset (entry 13).
- WRONG: `points[-1]` is the end of a path (entry 8).
- WRONG: 60 m garden strips would fit a 2 m cut on this slope (entry 5).

## Typed coordinates that remain, and why

- `stream_line`, `stream`: 5 and 4 vertices. A path cannot be derived from the height field or tied to a flood.
- `tarn.at` and `tarn_y`: a flood needs `at`; `at` cannot hold a formula (SPEC 12.6b).
- `court.region`: 6 vertices; the one authored decision everything in the compound hangs on.
- `bridge.path`: 2 points. `road_up`: 5 switchback vertices. `approach`: 1 point (the gate), which must
  agree by hand with the court's first and last vertex.
- `bm_over`: an overview camera.
Everything else (26 placements and features, the six garden plots, the walk, the lamps) is relative.
