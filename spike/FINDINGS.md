# Spike findings

One world ("Vale", 512 m square, 0.5 m samples) written as one TOML file, compiled by a Python compiler,
loaded at run time by a thin Godot 4 viewer, rendered from six bookmarks, then put through six changes.
Numbers below are from that one world on one machine. They are observations, not benchmarks.

| | |
|---|---|
| World source | 1 file, about 400 lines |
| Compiler | about 1,150 lines of Python; 8 s per compile |
| Viewer | about 700 lines of GDScript and six small shaders; nothing imported by the editor |
| Compiled world | 1025 x 1025 posts, 8 ground materials, about 4,800 instances (4 hand-placed) |
| Review | 6 bookmark renders at 1920 x 1080, about 40 s when the display is awake |

Not verified: the walk mode was benchmarked and captured at each bookmark but never driven by hand
here. Memory was not measured. Unreal was not touched. The winter variant changes the snowline, grass
and sky only: trees stay green and the lake stays liquid, because swapping assets per variant was not
built.

## Answers

### 1. Authoring surface: plain declarative data is not enough

The first field needed an expression. Within the first hour the source also needed named
sub-expressions (`let`), references from one field to another (`f('height')`, `slope('height')`), and
references from a field to a feature (`dist('valley')`). TOML carried the structure well and the
expressions badly: they are opaque strings, evaluated here with Python's `eval`.

**Locked:** the format is declarative structure plus a real expression language over fields.
**Open:** what that language is, how it is made safe and fast, and whether an SDK that emits the
document is the better way to write long expressions.

### 2. Seams: declared effects with limits worked, and stayed data

A seam was modelled as a **kind**: a named bundle of effects a feature has on fields. Each effect is
`field`, `value`, `mask`, a `reach`, and optionally a limit. Road, lake, cliff, building pad and
"a viewpoint is a clearing" were all written this way in the world file. None of them needed compiler
code of their own. This is the strongest result of the spike: a first-class concept can be a library
of kinds rather than a feature of the compiler.

What the compiler caught from those declarations, without being told about roads or lakes:

| Diagnostic | Raised when |
|---|---|
| `seam.change_exceeds` (error) | a pad or a lake bank would move the ground more than its limit |
| `seam.grade` | a path climbs more steeply than its kind allows after grading |
| `seam.relief` | the ground under a footprint varies more than the asset tolerates |
| `seam.junction` | a path that declares it joins another has no end on it |
| `effect.unbounded` | an effect's mask has not reached zero at its reach, so it would leave an edge |
| `place.overlap`, `place.avoid` | footprints collide; a hand-placed instance stands where a field says keep clear |

Where the model was pushed past what it had:

- **Order between kinds matters.** A road regrades the edge of a pad beside it; `cottage_c` kept a
  relief warning for the whole spike because of that. Declared order is not a real answer.
- **Effects need values derived from other fields.** A lake's water level depends on the finished
  ground at its rim. That needed a new function (`under`) and the limit needed a "where" clause.
- **A heightfield cannot be a cliff.** The cliff kind makes a steep step and the ground shader hides
  the stretch. There are no overhangs and no cliff meshes. This seam is untested.
- **Shaping terrain is still authoring.** The compiler refused a lake on sloping ground. The fix was a
  hand-written basin in the height field, which then pushed scarps into the hamlet, which the compiler
  also caught. The loop converged in two rounds, but nothing shaped the ground for me.

### 3. Engine handoff: run-time loading of plain data was enough for a viewer

The engine received rasters (height, water, grass density, material weights), one explicit instance
list, features passed through with their attributes, and bookmarks. It imported nothing.

The boundary this drew is sharper than expected. **Almost everything that made the picture believable
was on the engine side or in the asset pack**: the ground shader, the grass, the water, the light, the
tree models. The compiled world decided where things are and where they meet. It did not decide how
they look.

Two things crossed the boundary in a way the six concepts did not predict:

- **Dense groundcover is not instances.** Grass left the compiler as a density field and the engine
  realised it around the camera. A real production does the same with its foliage system.
- **Some assets are engine-side generators.** The only wilderness-grade trees available were built by
  a script at run time. The pack had to grow a third source type, `recipe`, whose size and cost are
  declared because the compiler cannot measure them.

### 4. Agent loop: diagnostics and renders did different jobs, and both were needed

About six render rounds took the world from first compile to the final base. Sorted by what found each
problem:

| Found by a diagnostic | Found only by looking at a render |
|---|---|
| trail too steep | "trees" bound to garden trees standing in stone planters |
| pad cut too deep after the basin edit | the lake sitting in a crater-like dam |
| 21 chunks over the triangle budget after the pack swap | untextured logs rendering white |
| viewpoints with a tree in the first metres of the view | shadows far too blue |
| lake banks too high (after I added the limit) | crease lines in the terrain (seen on the minimap) |

Two of the left-hand diagnostics (`bookmark.crowded`, the lake bank limit) did not exist until a render
showed the problem. So the loop works, but diagnostics are learned from renders, one failure at a time.
The minimap was the cheapest review tool: it comes out of every compile with no engine.

### 5. Concepts: six held, a seventh was needed, the asset contract took the most strain

| Concept | Verdict |
|---|---|
| Frame | Held. Feature-local frames (`on = { feature, t, offset }`) were essential, not optional. |
| Field | Held and carried more than planned: height, materials, water, densities, keep-clear, grass. |
| Feature | Held. Unknown kinds pass through untouched (the spawn zone). |
| **Kind** | **New.** The seam model. Not in the original six. |
| Instance | Held. An instance with a kind lends its footprint to that kind as a region. |
| Asset | Held with strain. See below. |
| Variant | Held. One overlay mechanism served the winter variant and every change file. |
| Producer | Held for scatter only. Its output is indistinguishable from hand placement downstream. |

Additions made under pressure during the spike, each a sign of where the model is thin: `let`
bindings, `under()`, `check_where`, `joins`, `recipe` assets, per-material `tint`, features positioned
`on` another feature.

**Frames have two real defects.** A path's local frame is discontinuous along its medial axis, so
`along()` or `sdist()` used far from the path put a visible crease in the terrain. And only whole
features can be placed in another feature's frame; a path's own vertices cannot, so moving the road
left the trail's start 31 m away from it.

**The asset contract is where a studio will feel this.** In one small pack there were three source
types (file, primitive, recipe), two scale conventions (unit-normalised and metres), two origin
conventions (centre and base), and three material conventions (embedded, a texture beside the file,
none). The facts the compiler needed per asset were: fit, lift, footprint as distinct from canopy,
sink, relief tolerance, solid or not, fields to avoid, triangle count, fallback colour. An ID and tags
cannot say what an asset *is*: the wrong "tree" passed every check.

### Frame rate: the first handoff made culling impossible

The first walk mode ran at about 12 fps at every bookmark, the same facing forest or sky. Measured at
1600 x 900 on one machine, with the same build:

| | Mean over six bookmarks |
|---|---|
| One world-wide batch per asset, full-detail terrain sheet, 420 m shadows | 11.9 fps |
| Instances emitted per chunk, with the viewer changes below | 114 fps (display-capped near 120) |

The compiler change was one line of grouping: instances per asset *per chunk*. Everything else was
viewer work: per-chunk batches the engine can cull, a draw distance from each asset's declared height,
no shadows from small things, the recipe's own far crown beyond 70 m, terrain in per-chunk pieces at
three resolutions, ground materials sampled only where present, 170 m shadows, no high-DPI rendering.
The changes were applied together, so their individual shares are not known.

The lesson is about the handoff, not the viewer. A flat instance list forces the worst case on any
engine. And the compiler had already said so: after the pack swap it flagged 21 chunks over the
triangle budget, one 21,700-triangle bush accounting for most of 46 M source triangles.

### Derived features and authority (added after the first pass)

The first lake was an ellipse the terrain was forced to fit. It was replaced by two lakes of the same
kind with opposite **authority**:

| | Tarn (scenery) | Mill pond (landmark) |
|---|---|---|
| Authority | derived: it conforms to the ground | anchored: the ground conforms to it |
| Authored as | a point, `level = "spill"`, a fence radius | a nine-point polygon |
| Shoreline development (1.0 is a circle) | 1.44 | 1.10 |
| Area, shore length | 5,062 m2, 364 m | 524 m2, 89 m |
| Beside it | a POI and a bookmark `on` its shore, resolved after the flood | a building `on` its outline, 12 m out |

What this settled:

- **A field can produce a feature.** The flood is the cells below a level that connect to a point,
  inside a fence. Its shore is a contour of the height field, and other things can be placed in that
  shore's frame. The steep east bank turned to rock through the existing slope rule, with no new
  authoring.
- **Authority replaced declaration order.** Effects now apply derived, then shaped, then anchored.
  Making building pads anchored removed the `cottage_c` relief warning that had stood since the
  first compile: the road no longer regrades a pad.
- **Two anchors that meet are reported.** The mill's pad and the pond's bank overlapped 0.4 m apart
  and the compile failed with `authority.conflict` until the mill declared `over = ["mill_pond"]`.
- **The cycle is a real rule.** A derived feature's effect on the field it derives from is skipped,
  and an instance standing on a derived shore may not reshape that field (`authority.cycle`).
- **Measurements steer.** Each compile prints area, shore length, shoreline development, island
  count and depth for every water body, so "more interesting" has a number between renders.

What it cost: placement became two-phase (things on a derived feature wait until the terrain exists),
and shaping the hollow the tarn sits in was still hand-written noise in the height field.

### Volumes: what one height per point cannot hold (second world, "Coast")

A second world, 384 m square: a beach cove, open sea, a headland with a sea cliff, and a cave through
the headland at sea level. One compile is 6 s.

A **volume field** is a field over x, y and z that exists only inside the box of a feature whose kind
hosts it. Positive is solid, and its base is the ground itself (`height - z`). The cliff's roughness
and wave-cut notch, and the cave (a tube along a 3D path), are effects on that field, written as
kinds in the world file like every other seam. The compiler meshes the result: 166 x 150 x 72 cells
of 0.75 m gave 96,162 triangles, with open space under solid ground over 10% of the footprint.

What held:

- **The same model reached 3D.** Field, feature, kind and effect needed no new concept, only a field
  with one more axis and a bounded domain.
- **The seam with the terrain is exact by construction.** At the box's sides the volume is blended
  back to the ground over an apron, so the mesh meets the heightfield with no gap. The terrain is
  cut away under the box with a hole mask.
- **Everything else kept working** because the top of the solid is written back into `height`.
  Grass, trees and bookmarks stand on the headland as before. A second raster, `bed`, holds the
  lowest floor, and the sea and the walk mode use it to get under the rock.
- **The sea needed no vocabulary yet.** It is the water field's base value, and the viewer extends it
  past the world's edge on a hint.
- **Diagnostics transferred:** air reaching the volume's floor, solid reaching its ceiling, and a
  cave path leaving its volume are all reported.

What did not:

- **Two rasters are not a general answer.** `height` and `bed` describe one overhang per column. A
  cave above a cave, or a bridge over a cave, has no representation for standing, water or placement.
- **Nothing can be placed inside.** Instances are grounded on `height`, so a chest in the cave is not
  expressible. Only a bookmark got an `under` flag.
- **The volume is not partitioned or budgeted.** One mesh, no chunks, no detail levels, not counted
  in the per-chunk budget.
- **Rock is one material.** Faces and ceilings take a single rock layer; ceilings show stretched
  texture. Strata, moss and wet rock would need a material field over the volume.
- **The effects ignore authority.** Volume effects apply in declaration order, as fields did before.
- **It is the compiler's mesh.** An engine with its own voxel or cave tooling would want the density
  field, not triangles. Which of the two we hand over is undecided.

### Working from reference images (first as two worlds, then merged into "Coast")

Two photographs were given as targets: a limestone coast with sea stacks at low sun, and a lighthouse
on a narrow basalt headland. Each became a world of about 150 lines, in two rounds of render and
compare. Both include a shared library of kinds (`worlds/lib/shore.toml`).

How close they got, judged by eye against the photographs:

| | Stacks | Lighthouse |
|---|---|---|
| Composition (what is where) | close: beach, a line of stacks, cliff receding on the right | close: headland from above, station at the point, benches stepping down |
| Landform character | weak: stacks are barrels, beds are even stripes | weak: banded faces, no columns; headland too broad and flat |
| Materials and colour | partly: ochre beds, warm light, wet sand missing | partly: dark rock, dry grass; no lichen, no white surf line |
| Structures | none needed | boxes and a primitive tower stand in for the station |
| Viewpoint | wrong height: from the beach, not the cliff top | roughly right, lens too wide |

What the image-led start showed:

- **Composition was the easy part.** Reading "what is where" off an image and writing features for
  it worked in one pass. The framework is friendly to that.
- **Character is the hard part, and text does not carry it.** "Barrel" versus "weathered stack" is in
  the profile of a shape, and the only tools were noise amplitudes. An agent can see the difference
  in a render and cannot say it in the source.
- **Viewpoint matching was done by guessing.** A bookmark has a position and a target; a reference
  image has a horizon line, a focal length and anchor points. Nothing connects the two.
- **There was no comparison tool.** Render and reference were compared by looking. Nothing measured
  palette, horizon height or silhouette.
- **`[look]` became a junk drawer.** Sun, sky colours, fog, surf, water colours and rock tints all went
  into one untyped table that the compiler passes through and only the viewer understands.
- **Kinds needed sharing at once.** The second and third coastal worlds would have copied sixty lines
  of kinds, so `include` was added.
- **A cap on effects was needed.** Roughening a cliff face grew spikes above its rim until effects
  were limited to the height of the nearest crest. Another rule the author should not have to know.

Assets and textures that were missing to match the references: layered sandstone and columnar basalt
rock materials (the tint over one grey rock texture is a stand-in), wet and dry sand, surf and foam
with breaking waves, a cloud layer and a lit sky, a lighthouse tower, keeper's houses and a wall kit,
rubble and fallen blocks for cliff feet, and dry coastal grass and heather.

### Merging separately built pieces into one map

The stacks and the lighthouse point were first built as two standalone worlds, then folded into
Coast by hand. Coast went from 384 m square to 704 x 448 m, three volumes, 17.6 s per compile.

What the merge took, all of it manual:

- **Every coordinate was retyped.** Stacks, the point's outline, buildings and bookmarks were all
  moved by re-deriving numbers. Nothing could be placed as a unit.
- **The coastline was welded by editing one polygon.** The point's outline was pasted into the
  mainland's vertex list. That is the actual seam, and it was done by eye.
- **Height could not be composed.** Each world had its own formula for the whole height field. The
  point's benches had to be rewritten as a layer inside Coast's formula.
- **Global tables collided.** One `[look]` per world meant the point lost its dark basalt and took
  Coast's ochre beds. Seeds are global too, so the stacks do not look as they did standalone: noise
  is seeded by the world and evaluated in world coordinates.
- **Kinds had to be reconciled.** Coast's own rockwork and the library's had the same name and
  different attributes; one was deleted.
- **A compiler bug appeared.** No single world had held two volumes, and each one's effects were
  being applied inside the others.

What helped: the pad limit caught a building placed on a bench step, and the blocked-view warning
caught the long-lens bookmark aimed through a ridge.

### Reachability lint (on demand)

`--lint reach` checks only what the world states. A world declares a movement profile (slope limit,
wading depth) and expectations: "from here, these are reachable by this profile", or the reverse,
"these must not be". With no profile or no expectation it checks nothing.

On Coast it reported, before anyone had told it where to look, that the lighthouse was cut off by
12 m of ground at 78 degrees (a bench step) and that the first headland's top could not be reached
either (4 m at 38 degrees). Three graded roads later all expectations hold, including the negative
one that the stacks stay out of reach.

Its limits are the limits of its claim. It reads the terrain's top surface at 1 m, with solid
footprints as obstacles. It does not see caves, interiors, bridges, stairs, fences without a
footprint, doors, or anything the engine's collision adds or removes. "No route" is therefore a
strong statement about the ground and "a route exists" a weak one about the game.

### Interiors, first pass (world "Station", from a concept image)

A cutaway concept image of a lighthouse and keeper's cottage was generated, then written as a world:
four round tower rooms on four levels joined by three spiral stairs, and a cottage of four rooms
joined to the tower by a passage. 300 lines of source, 0.1 s per compile, five bookmarks and a floor
plan image per level.

How it mapped onto the model:

| Building idea | Expressed as | New? |
|---|---|---|
| Level | a named floor height: a frame | a third kind of frame, after the world's and a feature's |
| Room | a region feature on a level, of a kind that `encloses` | no |
| Door, window | a point feature `on` a room's outline, with width, sill and head | no |
| Stair | a feature that links two rooms on different levels | no |
| Walls, slabs, steps | built by a producer from the rooms, as scatter builds instances | the producer is new; its output is generated mesh, not instances |
| Furniture | an instance `in` a room, grounded on that room's floor | grounding on something other than terrain |
| A book on a desk | an instance `atop` another instance, in that instance's frame | yes: instances as frames |

What worked:

- **The checks were the most useful part.** On the first compile they caught a barrel standing in a
  door's swing, two bunks through a wall, and a front door 32 cm above the ground outside. Also
  written: sealed rooms, doors that lead nowhere, stairs through walls, stairs too steep.
- **The floor plan is the interior's minimap.** It comes out of every compile with no engine and was
  enough to review the layout.
- **Rooms that touch share one wall** because walls are the union of the rooms' outlines, not one
  ring per room.
- **Layout from the image came across in one pass**, as it did outdoors.

What did not, and why this looks like its own domain:

- **It is a different vocabulary with its own standards.** Storey, space, wall, door, stair and slab
  are defined terms in building-information modelling. The spike reinvented a thin slice of them.
- **Character is absent.** The concept's gabled slate roofs, banded tower, lantern glass and chimney
  are not layout; the render is a white cylinder and flat-roofed boxes. That is art, and it arrives
  as assets or a kit, not as rooms.
- **No kit was tested.** Walls are generated mesh. The one kit check (do wall lengths divide by a
  module width) fired on three of eight rooms, which says a kit contract would constrain room sizes
  from the start. Round rooms were not checked against a kit at all.
- **There is no hand-authored escape hatch.** Every wall is parametric. A studio will have rooms that
  are a single modelled mesh; the boundary for those (footprint, openings, floor heights) is the part
  we would need to own.
- **Dressing stops at one book.** Props on furniture work through `atop`, but nothing says what
  belongs on a desk, and surfaces are not declared by assets.
- **The heightfield is irrelevant indoors, and the outdoor tools with it.** Walking, the reach lint,
  budgets, water and the cutaway view all assume one ground surface. The cutaway hides built levels
  but not the furniture on them.

A walking body with collision was added to the viewer afterwards, and it found a layout fault the
compiler had missed: the body climbed the first stair and stopped at 1.98 m because a bench on the
floor above stood over the stairwell. A check for furniture on a stair or over its well was then
added and reported three such pieces. A scripted walk test now passes: a wall stops the body 0.31 m
short (its own radius), and the first stair carries it from 0.30 m to 3.52 m against a floor at
3.55 m. Lesson: a simulated walk finds what static checks were not written for, and each find
becomes a static check.

**Dressing at scale.** The station went from 11 placed things to 138 across eight rooms (0.2 to
0.9 per square metre). Placing each by coordinate would not have scaled, so four placement tools were
added, none of them a new concept:

| Tool | What it does | Built on |
|---|---|---|
| `against` | stands a thing with its back to a wall, inset by half its own depth | a room outline as a frame |
| `mount` | hangs a thing that far above the floor | grounding on a named surface |
| set | a group of items in one local frame, placed as a unit (a table, its chairs, its plates) | a part, at furniture scale |
| run | repeats one asset along a stretch of wall | a producer |
| dress | strews small things on top of something already placed, seeded | a producer, on an instance's top |

The first compile reported 12 errors. Half were layout mistakes (a rug over a stairwell, a hearth and
a bookcase sharing a wall, a room with no free wall left). Half were gaps the tools then had to close:
round furniture treated as square, straight furniture poking through a curved wall, a mat counted as
blocking a door, and a run packing four barrels where three fit.

Three instances carry game data the compiler does not read (`interact`, `hides`, `text_id`): a
painting, a ledger among strewn books, and the log on the desk. They reach the engine by ID with
those attributes untouched.

The boundary this suggests: the core owns levels as frames, grounding on a named surface, the
threshold seam between a door and the terrain, and the footprint a building claims on the map. Rooms,
walls, stairs, kits and dressing belong to a building domain built on top, as vegetation will need
its own.

### A building layout (world "College")

A new hillside college, 320 m square: a gate, a cloistered garden court, a paved main court above
it, a hall on the highest terrace, a wing along the main court, two dormitory rows up a lane, houses
by the approach. 63 hand-placed things and 29 kit bays in six runs, about 330 lines of source,
1 s per compile. It builds in boxes with the committed pack and with real buildings and a real
modular kit through the local one.

The first compile reported 20 errors. They sorted into three kinds:

- **Anchors on anchors (11 errors).** Every building standing on a terrace conflicted with the
  terrace, because both are anchored and declaration order put pads first. Fixed with one rule:
  among anchors the more specific wins, so the smaller region is applied after the larger one and
  takes its level. After that only two real conflicts were left (a wing half off its court, two
  gate buildings on a slope), and both were layout faults.
- **Kit pieces reported as overlapping (10 errors).** A dormitory bay measures 7.06 m and repeats
  at 7 m. Neighbours in one run are meant to meet. They now share a group and are exempt.
- **Kit runs that do not close (2 errors, both mine).** "A run of 50.2 m takes 4 bays of 12 m and
  leaves +2.2 m; 48 m or 60 m would close." This is the kit seam as a compiler error.

What it showed:

- **A kit piece's bounds are not its module.** The cloister bay's box is 6.44 m and its pitch is
  6 m. The pack had to declare `module_m`, and I set those three values by judgement from the
  bounds, not from any documentation of the kit. They are unverified.
- **A kit run is a producer** like scatter: two points and a bay lower to instances, plus one pad
  under the whole run so its bays share a level.
- **Some buildings are walked through.** A gatehouse on the path made the reach lint fail until its
  asset was marked not solid. Arcades and gates need a passable footprint, which is neither solid
  nor empty.
- **Authority also orders paint.** A lawn inside a paved court vanished until it was anchored too.
- **The reach lint held on a real layout:** nine expectations, one per entrance, all true after the
  fixes.

**Dressing the college.** A second pass took it from 131 placed things to 354 (257 of them
explicit, the rest scattered trees): hedged lawns let into the courts, lamps and benches around
every court, an avenue with banners at the approach, a market by the houses, an orchard, urns along
the hall terrace, planters either side of eight front doors, and a kit end and two corners. It added
about 230 lines of source. Two small tools were needed: placing a thing `beside` a building in that
building's frame, and a producer that flanks a list of front doors with a pair of planters. The
first compile reported 14 errors, 12 of them one gap: neighbours in a run of hedges, and a kit end
beside its run, were flagged as overlapping until runs shared a group as kit runs do. The real pack
now totals 7.1 M source triangles; the frame rate was not measured.

Not tested: a run that steps with the slope, stairs between
courts (they are graded ramps), retaining walls, and whether the bays actually join without gaps at
their declared pitch. The renders look continuous from the bookmarks, which is not a measurement.

### Layers (the college, split by concern)

The college's one 560-line file became a 51-line master and five layer files, lowest first:

| Layer | Lines | Placed | Features | Holds |
|---|---|---|---|---|
| terrain | 124 | 0 | 12 | fields, courts, ways |
| buildings | 103 | 44 | 0 | hand-placed buildings and kit runs |
| dressing | 302 | 213 | 7 | lawns, hedges, lamps, market, planters |
| people | 75 | 10 | 0 | NPCs and where each must be able to walk |
| review | 54 | 0 | 6 | bookmarks |

They compile as one world (1 s, 0 errors) and each leaves the compiler as its own file
(`layer_people.json` and so on) beside the combined output.

What held:

- **An NPC is an instance with a marker asset and data the compiler does not read.** `role`,
  `schedule`, `home` and `route` pass through untouched. Its placement uses the same frames as
  everything else: beside the gatehouse, beside a stall, on the dormitory lane, around a lawn.
- **Its rounds are reach expectations in its own layer.** "The porter's rounds", "the baker walks
  home", "the watch's patrol": seven expectations, all checked against the map with the on-foot
  profile. With the map's own nine, 16 of 16 hold.
- **Direction is enforced.** A building placed beside a market stall was refused: "it is in layer
  'buildings' and refers to 'stall_a' in the higher layer 'dressing'".
- **Lists join, tables merge.** Layers add to the world's lists; an ID defined in two layers stops
  the compile.

What did not:

- **Direction is only checked on placement references.** Expressions that name a feature
  (`dist('x')`) are not checked, and neither are kinds, fields or sets, which are still global.
- **Lower layers are affected by upper ones through effects.** The dressing layer's lawns repaint
  the terrain layer's ground. That was allowed, and nothing declares which effects may cross.
- **IDs are still one global namespace**, so a baker and a building could collide.
- **No layer can be compiled alone or swapped**, and there are no per-layer variants.
- **The NPC markers are drawn in every render**, because the viewer has no notion of a layer to hide.

### Parts (the college and the station placed onto Coast)

A part is compiled alone first, in its own frame and with its own seed, then placed in a master. Two
were placed onto Coast, one of each boundary kind:

| | College | Station |
|---|---|---|
| Boundary | seam: its fields replace the master's inside a footprint, blended over a 30 m apron | portal: its front door is put at a point; nothing is blended |
| Placed | 364 things, lifted 11.9 m, moved by (360, 278) | 138 things, lifted 30.5 m to the point's bench |
| Master had to add | a rise under it and a link road to its arrival point | a level pad under it |

Coast now compiles in 29 s with 1,873 placed things, and 22 of 22 reach expectations hold: the
master's six and the college's sixteen, which came along under the part's name.

What the first compiles reported, each a real consequence of placing a part:

- **The part cut the master's own route.** The college's seam put a 74 degree bank across the road
  to the lighthouse. The master's reach expectations caught it.
- **A hillside does not sit on flat land.** With the college joined at its arrival point, its far
  edge stood 24.5 m from Coast's ground. The master needed a matching rise; the last 10.6 m was
  accepted by raising the limit on purpose.
- **The interior landed across a bench step**, with 7.6 m of relief under its footprint.
- **Materials overflowed.** Master plus part made nine ground materials and the output holds eight;
  the college's paving rendered black until a check was added and one material dropped.

What held: scoped names (`college.hall`, `station.kitchen`), a part's own bookmarks, diagnostics,
layers and expectations arriving in the master, and a declared joint (`joins_at`) deciding the lift.

What did not:

- **No rotation.** Parts are only translated, by whole grid cells.
- **The master had to be shaped by hand** to receive the college. Nothing derives the rise a part
  needs from the part.
- **Open defect: the terrain still shows through the station's floor.** A hole mask is written under
  its rooms and reaches the output, but the render is unchanged and the cause was not found.
- **Brought instances are not checked against the master's** for overlap.
- **A part's kinds and fields are baked**, so the master cannot see or adjust them, and a part
  cannot be compiled once and reused: each master compile recompiles it.

### The player-facing map (Coast)

A study came first (`reports/Game map production pipelines.md`): wherever a shipped pipeline is
documented, the map is derived from the level's own data and only a small style layer is made by
hand, and regeneration that overwrites hand edits is the recurring failure.

The spike then split the map the same way. `--map` emits, for Coast (1409 x 1121 px at 2 px per m):
a class per cell (deep, shallow, sand, grass, forest, rock, paving, road, building), a flat class
picture, a plain base picture with relief, 31 building footprints, 7 icons, 1 region label, the
world-to-map transform, and a fingerprint for each of 64 tiles. An image model then repainted the
map as a watercolour atlas, once from the base picture and once from the flat class picture. Icons
and labels were drawn from data on top afterwards.

| Measure | From the base picture | From the flat class picture |
|---|---|---|
| Shore, data to picture, mean | 0.9 m | 0.8 m |
| Shore, data to picture, 95th percentile | 3.8 m | 3.6 m |
| Stretch to fit (the model returned 1152 x 928) | 1.2% | 1.2% |
| Building contrast with surroundings (of 441) | 74 | 101 |
| Road contrast with surroundings (of 441) | 17 | 6 |

One image per variant, one model, one prompt. These are single observations, not a result.

What held:

- **The coastline stayed put.** The true shore lies within about a metre of a painted shore on
  average, at 2 px per metre, in both variants.
- **Icons and labels from data land where they should**, by eye: the lighthouse on the point's tip,
  the landing at the road's end, the college over its courts. The model was told to paint no text
  and painted none.
- **A level change invalidates little.** Adding a sea stack and moving a marker changed 1 of 64
  tiles; the marker moved with no repainting at all.

The same map runs in the Godot walk mode: a north-up minimap centred on the player, and a full map
on a key. The viewer loads the painted picture and `map.json`, and draws the player, icons and
labels from data through the world-to-map transform; the picture is 1152 x 928 and the data is
1409 x 1121, and the transform absorbs the difference. In one capture at the cove the arrow sits on
the beach beside the headland, which is where the camera is. Coast ran at 22 fps in that capture.

What did not:

- **The model invented a feature.** From the base picture it drew a wall around the college, where
  the relief shading showed the square edge of the part's footprint. The flat class picture has no
  relief and produced no wall. A seam we left in the terrain became a thing on the map.
- **Roads nearly vanished** in both, more so from the flat picture (contrast 6 of 441).
- **The water measure failed.** Overlap came out at 0.64 and 0.61, and the reverse shore distance
  at over 100 m, because pale sea washes and wave lines were classed as land by a two-colour
  classifier. That is a fault in the measure, not evidence of drift, and those numbers should not
  be used. Only the data-to-picture shore distance is trustworthy here.
- **Nothing was measured for forests, footprints' positions or roads' positions**, only contrast.
- **Tiles were fingerprinted but not repainted.** Whether one tile can be repainted to match its
  neighbours' style is the untested half of surviving regeneration.

## The change scenario

| Change | Edited by hand | Followed automatically | Compiler reported |
|---|---|---|---|
| 1. Swap the asset pack | one flag | every instance, material and bookmark | 21 chunks over the triangle budget (0.98 M to 46 M source triangles) |
| 2. Move the road | 1 path | terrain cut, ground paint, vegetation clearing, 4 buildings, 3 bookmarks, 1 POI | 2 pads now too deep to cut (errors), trail no longer joins the road |
| 3. Larger building | 1 asset ID | pad, paint and clearing grew with the footprint | nothing new |
| 4. Add a cliff | 1 feature | forest thinned on the face, boulders at its foot | trail grade 38% to 55% where the cliff crosses it; a bookmark's view blocked by terrain |
| 5. Winter variant | 0 (overlay) | snowline, grass, sky | nothing new |
| 6. Invalid edits | 6 on purpose | none | 4 errors and 2 warnings; all six caught, including a field cycle |

Change 2 is the one to remember. Everything placed *relative* to the road moved with it. The one
thing placed in absolute coordinates beside it (the trail) broke, and was only reported because the
trail had declared its intent to join.

## Decisions this locks

1. The source is declarative structure plus an expression language over fields. Plain TOML alone fails.
2. Seams are declared effects with limits, kept as data. New concepts arrive as kinds before they are
   considered for the compiler.
3. The compiled form is explicit: rasters, one instance list, features passed through. Producers lower
   to it and leave no trace.
4. The world names assets by ID. A separate pack binds IDs to sources and facts. A primitive pack that
   always builds is part of the contract.
5. One overlay mechanism serves variants and edits.
6. Bookmarks and static budgets are ours. The minimap is a free by-product of compiling.
7. The engine owns the look. We own where things are and where they meet.

## Still open

- The expression language, and the authoring ergonomics around it.
- Ordering between kinds, and values shared across effects of one feature.
- Relations between path vertices (junctions, networks), and the medial-axis defect in path frames.
- Anything that is not a heightfield: cliff meshes, overhangs, caves, bridges.
- The contract for engine-side realisation of fields (grass) and for recipe assets.
- Scale. The compiler evaluates every field over the whole world at once. A 4 km world at this
  resolution is 64 times the samples; it needs tiling and incremental rebuilds.
- Implementation language. A Python spike says nothing about it.
- Unreal, streaming, navigation, gameplay beyond passing features through.

## Seeds for the ledger

Terms the spike ran into, each of which should get a row: heightfield, splat map, triplanar mapping,
cut and fill, road grading, retaining pad or skirt, shoreline blend, keep-out mask, scatter with
rejection, foliage system versus instanced meshes, asset pivot and scale conventions, LOD chain,
impostor, per-chunk budget, spawn volume, viewpoint or vista, variant or season layer, medial axis.

## User code joining a build (SDK spike, 2026-11-12)

Asked: can a user's own code shape a world through a contract of plain data, with an SDK as a
convenience over it? Built on this compiler, since the public contract is the part that matters and
the engine's language is not.

**What was built.** `[[step]]` rows name a command and a point. `generate` writes a layer, before
the world is compiled or after a named layer; `lint` reads the finished build. The compiler hands a
step one JSON request and takes back files: `layer.json`, rasters of 32-bit floats, and
`diagnostics.json` (`worldc/steps.py`, 145 lines). A field may now be read from a file.
`sdk/viasdk` (135 lines) reads the request and the build as numpy arrays and writes the answer; it
holds no engine logic. The `massif` world uses all of it: its land is 99 lines of the user's
Python (FFT noise, stream-power erosion with scikit-image's pit filling), a second step weathers
the ground after the terrain layer and writes a `talus` field, and a third is a project rule
("every home within 30 m of a road"). The compiler's road, pads, ground paint and scatter run on
top. The six earlier builds are byte for byte unchanged.

**What held.**
- Plain data was enough at these points. Nothing needed a call back into the compiler.
- A generated layer is a layer. It needed no new concept: the step's stream (`beck`) is a feature
  the hand-written layers measure from, and its `wet` and `talus` fields drive ground paint and
  scatter like any declared field. "Refer down, never up" placed the steps without a new rule.
- User diagnostics read like ours: `warning homes:far_from_road high_barn at (150, 150)`.
- Skipping a step when nothing it reads has changed is small: a hash of the command, parameters,
  named files, frame and the build handed over. A parameter change reran only that step; moving
  the lane reran the step above the terrain layer and not the one below.
- A failing step is reported with its last lines, its stale output is removed, and the layers
  above then fail on what is missing, which reads correctly.

**What did not hold, or was learnt.**
- **A step that returns a changed field makes the lower effects apply twice.** The weathering step
  reads a height that already has the road in it, and the compiler then applies the road again.
  For a road that is nearly harmless. For an effect that adds (a cliff's `v + drop`) it would
  double. The contract must say whether a returned field replaces the result so far, or is new
  ground that everything is applied to again. This is the main open design question.
- **The author cannot draw on land that does not exist yet.** The lane's coordinates were written
  after compiling once and looking. Generated terrain makes hand-placed coordinates fragile: a new
  seed moves the valley and the lane stays. Things placed on generated land want to be placed
  relative to what the step hands back (`on` a feature it wrote), or written by the step.
- **Generated documents should be JSON.** Python has no TOML writer in its standard library. The
  loader now takes either: TOML from people, JSON from programs, one data model.
- **A step sees only what a build exports.** `build.field("forest")` fails unless the world lists
  it under `[export]`. Whether a step gets every field, or only declared ones, is undecided.
- **The cache covers inputs, not outputs.** A generated raster damaged by hand was reused and
  caught only by the size check. And working out the key for a step above a layer means compiling
  that layer every time (1.4 s of a reused 1.4 s build here).
- **Undeclared inputs go stale unseen.** A step that reads another file, the clock or the network
  must list it under `inputs`; nothing enforces that.
- **Arrays need a stated orientation.** Row 0 is the south edge. The SDK says so in one place;
  without it every user would find out by getting a mirrored world.

**Not attempted.** Hooks inside a layer's resolution (a custom effect or placement rule), a
long-lived host process, steps in parts, trust (a world's steps run unasked), Windows, and whether
a second machine reproduces the same rasters.

## Pictures as fields (2026-11-12)

Asked: an author already has a heightmap, a noise tile or a painted mask. Can the world take it in?

**What was built.** A field with `file` naming a picture reads it; no new concept was needed. The
keys are `channel` (r, g, b, a, luma), `range` (what black and white stand for), `extent` (where
the picture lies in the world), `outside` and `filter`. The picture is resampled to the grid
whatever its size, with its top row north. The field is then used by name like any other. The
`relief` world takes its height from a 16-bit heightmap at half the grid's resolution, its woods
and bare stone from two channels of one mask, and a hollow from a 64-pixel stamp placed in a
64 m square; a flood lake, a road, a house and scatter sit on top. Checked: the world's corners
match the picture's to a centimetre.

**Boundary.** The picture is the author's to get right. The compiler places and scales it, and
says what it can see: a missing or unreadable file, and an 8-bit picture stretched over a range
wide enough to show as steps (`field.image_coarse`: 256 levels over 66 m is 0.26 m a level).

**Learnt.**
- "Image input" and "what a step hands back" are the same feature. Both are a field read from a
  file; a step's `.f32` is the exact form and a picture is the convenient one.
- One picture, several fields: each channel is declared as its own field. That is wordy for a
  four-channel mask but keeps one rule.
- A picture cannot yet be a ground-material field directly; it is read as scalar fields that
  the material layers use as masks.
- Not handled: EXR and other float formats beyond what the imaging library reads, colour
  profiles, tiled pictures for worlds larger than one image, and pictures as engine textures
  passed through to the output.

## A town and a city district (2026-11-12)

Asked: build the dogfood town from the city study, then a larger district with a road network,
bridges, an overpass, sidewalks, a railway and a waterway. Reasoning and open questions are in
`docs/design/ways-and-assets.md`; behaviour is in SPEC section 12.

**Built.** `market`: a 256 m hill town, four streets, a square, 15 kit-run rows, 119 placed
things, written by hand. `city`: 1,280 m, a river, eleven streets, an expressway on a 440 m bridge
and a 220 m overpass, a railway with a bridge and level crossings, a ferry route on the map only, a
park, and 1,055 buildings in 295 kit runs laid out by a 100-line user step. It compiles in 22 s.
Compiler additions: kit runs `along` a street with weighted variants and an end piece; `order` on
effects; decks; `kinds/ways.toml`.

**Held.**
- Way types as kinds. The compiler learnt no new names; street, motorway, railway and waterway are
  data. A map-only waterway is a kind with a map style and no effects.
- Sidewalks as a zone of the street's cross-section. One feature, one move.
- A deck as a second surface carried as a mesh. The 2.5D height field did not need to change.
- The clearance check. A rail bridge under the water line was an error on the first compile.
- User code for placement rules. The lots step used a geometry library to test each footprint;
  the framework only placed the kit runs it was handed.
- Diagnostics as the working loop for layout. The town's three first errors were all fixed from
  the message and position alone.

**Did not hold, or learnt.**
- Effect order had to become explicit. Where streets met, one's pavement covered the other's
  roadway until effects carried `order`.
- Buildings fronting a square fought it for the ground's level until they stopped asking for
  ground of their own. A building's level is sometimes its neighbour's to give.
- A step could not read a street's width: builds carried feature keys but not kind defaults.
  Added behind `[export] feature_attrs`. What a build tells a step is still decided case by case.
- Every row is a flat pad. On the town's slope a long row would need to step; short rows with
  alleys hid it.
- Greybox reads as a city from the air and as blank walls from the street. The kit is the
  missing half, and the asset contract for it is still only proposed.
- Adding two surface materials to the shared pack changed `world.json` in every build, because a
  build lists the whole pack's surfaces, not the ones it uses.

**Not attempted.** Junction geometry, slip roads, tunnels, kit corners and filler pieces, blocks
and lots as concepts, the asset contract itself, and any real kit.

**Loop ramp (added the same day).** Asked whether the model could hold one. It could not: a deck's
ends rested on the ground only. With `from_on` / `to_on` (an end rests on another deck) and a grade
limit, the `city` world has a loop ramp from third avenue up onto the expressway, meeting the
overpass within about 10 cm. Lesson: the first answer given was an analysis; the demonstration took
less time than the analysis and found two over-steep bridges the analysis had not. Every request is
now logged in `docs/challenges.md` with whether it was shown.

## A city that makes sense (2026-11-12)

The first `city` was shown to make no sense: a uniform grid, a straight expressway at the edge that
no street reached, rings of buildings round empty blocks. Every check passed. It was rebuilt.

**How.** By hand: a river, eight arterials, two freeways on viaducts joined at a T, 23 points that
label districts (zone and grid angle) and 4 that mark interchanges. A road step (`tools/network.py`,
user code) derives districts as the faces between arterials, a grid of local streets in each at its
own angle and block size, a bridge where an arterial meets the river, and four ramps at each
interchange, a loop where asked. A lots step fills each block by its district's zone. Result: 159
streets, 26 districts, 18 ramps, 5 bridges, about 3,900 buildings, 0 errors, 70 s.

**Learnt.**
- Sense comes from structure, not from checks. What fixed the city was a hierarchy with two levels
  drawn and three derived. No diagnostic distinguished the two versions.
- Thirty-nine hand-written rows produce the whole city. That ratio is the authoring model worth
  keeping: draw the level you care about, label it, derive the rest, and keep the derived rows
  inspectable.
- All the road knowledge is in a step. The compiler gained one attribute (`settle`) and a kind.
- A deck's profile was too poor a second time. The southern freeway had to hold its height for a
  kilometre and come down at the map's edge; a straight line between its ends passed 1.7 m over an
  arterial. `settle` patches it. Profiles need a real description.
- A way cannot be interrupted. An arterial crossing the river is graded and painted across the
  bed, and the bridge is laid over it. It works to look at and is wrong.
- A step above a layer costs a compile of that layer. Two steps made the build three compiles.
- Labels as points worked well: a district's nature is one point with two attributes, and an
  unlabelled district is a warning with a position.
- Still boxes. Blocks are filled along their streets and empty in the middle; towers are sparse.

## A hard-surface stress test (2026-11-12)

Asked, as a stress test and not a use case: can the model hold a small mechanical object, a
sidearm, the way a modelling tool would, with constraints and without destroying earlier work?

**Built.** `worlds/sidearm`: 13 parts, each a feature, in 230 lines. One unit is a centimetre. The
object is the same kind of solid the caves are cut from. Seven kinds were written in the world
file: add a block, a rod, a ring; cut a block, a rod, a row of grooves; and the box of space it is
modelled in. A variant draws the slide back. It compiles in under a second with no change to the
compiler, and reads as what it is: bore, sights with a notch, grooves, trigger guard, grip.

**What carried over unchanged.** A part is a feature with attributes. An operation is a kind, as
data. Add and cut are effects on a field. Editing a number and rebuilding is the whole loop, and
nothing earlier is destroyed. A second state is a variant. This is the cave's mechanism at one
hundredth the size, and the fit was close enough that nothing had to be invented.

**What it could not do, and what each says about worlds.**
- *No frame inside a frame.* The slide, its sights, its grooves and its port are one moving
  assembly, and the variant moves six parts by hand. Nothing says "these belong to that". The same
  gap is why a placed part cannot be turned and a set cannot hold a set.
- *Relations point one way.* A part can stand `on` another. Two parts cannot agree on each other,
  as a slide and its rail must. In a world this is the road and the building that each want the
  other to move; authority is our answer, and it is a declared direction, not a solution.
- *Operations run in one fixed order.* All adds, then all cuts, by the order kinds are written. A
  modeller needs a stated sequence per object. The world side met this as authority versus order.
- *Every part repeats its numbers.* The slide is 20 long; the port, sights and grooves each state
  their own position. Nothing derives one from another. Worlds have `on` and `along` for this and
  volumes do not.
- *Sharpness is the cell.* Edges are rounded at 1.5 mm, and a thread or a knurl would need a finer
  grid everywhere. The world side is the rut narrower than a post.
- *Shape is lost at once.* Each part is exact as written and becomes samples immediately. A
  modeller keeps exact shapes until the end. So do we, for paths and regions, and not for solids.
- *It had to sit on the ground.* The modelling space had to reach below the terrain, and the
  object's top became "the ground" for the camera. A solid is not yet a thing apart from land.

**Verdict.** As a modeller it is an implicit one: fine for form, wrong for manufacture. As a test
it was worth more than expected, because each failure is a known world problem seen without the
clutter of a world: nested frames, mutual constraints, sequence, derived dimensions, resolution.

## The stress test pushed: a real object from its published numbers (2026-11-12)

Asked: take a real pistol, get its published dimensions, and model it without hand-placed
coordinates, letting the framework change where it must, with the reasons recorded. Then add
optional accessories.

**Built.** `worlds/pistol`: a service pistol's outside form from six published numbers (overall
length 219 mm, barrel 127 mm, distance between sights 165 mm, height 140 mm, width 33 mm, bore
11.43 mm; sources differ on length by up to 9 mm). 17 parts and 3 mounts. No part has a number
for where it is: each size and position is a formula over the published numbers, over a few
proportions marked as estimates, and over other parts (`= slide.top + hz`,
`= front_sight.cx - sight_radius`). Three accessories are separate files, each attaching to a
named mount: a reflex sight, a muzzle device, a light. A variant draws the slide back by setting
one number.

**Measured.** A lint in the world reads the built mesh and compares it with the stated numbers.
Built: 21.90 long, 3.30 wide, 14.00 high, against 21.90, 3.30, 14.00 stated. On the first compile
it reported the object 2.01 too long: the raked grip's heel stood out behind the frame's tail. The
fix was to restate one relation (place the grip from its heel), not to move anything.

**What the framework gained, and why each.**
1. *Attributes as formulas.* A string starting with `=` is a formula over the feature's own
   attributes, the world's parameters and other features' attributes; a kind's defaults may be
   formulas, so every block has `top`, `bottom`, `front`, `back` for others to refer to; a circle
   is an error naming its chain. Reason: without it "no coordinates" is impossible, and it is the
   cheapest honest answer to the previous test's "no frame inside a frame" and "every part
   repeats its numbers". The `=` also settles, for attributes, the old problem that TOML cannot
   tell a formula from a string.
2. *Kinds with no plan* (`plan = false`). Reason: every part carried a meaningless `at`.
3. *A solid apart from land* (`free = true` on the box). Reason: the object had to be sunk into
   the ground and its top became the ground. Both hacks are gone.

**Considered and left out, on purpose.**
- A stated sequence of operations per part. No part needed it: adds then cuts was enough.
- Units. One unit is still declared a centimetre by comment.
- Sharper edges. Still the cell.
- Turning a part. Formulas give one part's place from another's, not its rotation; a canted sight
  cannot be said.
- Two parts constraining each other. Still one way only; a circle is refused, not solved.

**Learnt.**
- *A mount is the connector the asset contract has been missing.* It is a named place worked out
  from the host's parts, and an accessory file refers to it and to nothing else. Three accessories
  attached with no change to the host file. This is how a kit's sockets should work.
- *An optional thing is an overlay.* The mechanism already existed for worlds; it needed nothing.
- *Derived attributes do most of what nested frames would.* Drawing the slide back went from six
  hand-edited parts to one parameter, because everything that rides on the slide is written
  relative to it. What they do not give is rotation.
- *Measure, then restate the relation.* The lint caught a wrong relation in one compile, and the
  fix was an edit to intent. That is the goal of the whole tool, seen at small scale.
- *Checks need to know the state.* With the slide back, or with accessories, the same lint
  reports the object as off its published size, which is true and not a fault.
- *This applies to worlds at once.* A building's door at `= wall.front`, a lamp spacing derived
  from a street's width, a platform height from a neighbour's: all are the same formula.

**Defect seen.** The render script hung twice when run for a second or third build in a row; the
pictures it had already written were complete. Not investigated.

## Mounts in a world: doors and bridge ends (2026-11-12)

Asked to go ahead with the two places the "thing with parts" framing was expected to pay off
first: doors that paths find, and things that meet end to end.

**Built.** A mount is a named place: declared on an asset (`[asset.X.mount.door]`), or the
`start` and `end` every path has. A path may say `starts_at` and `ends_at`. In `college`, three
buildings have a door and each has a walk that ends at it. In `example`, both lanes meet the
footbridge's ends by name. About 70 lines of compiler.

**Shown.**
- The chapel was moved 18 m and turned 60 degrees by an overlay of four lines. Its walk ended at
  the new door, within the sampling of the path, with nothing else edited.
- The footbridge was moved 12 m and lengthened. Both lanes followed, and the build had no errors.
- Removing a walk gave `mount.unmet: chapel: no path reaches its door`, with the door's position.
- A wrong name gave `mount.unknown ... 'hall' has: door`.

**Learnt.**
- *A door makes the ground depend on the building.* A path to a door refers upward if paths live
  in a terrain layer and buildings above it. The honest reading is that a building's walk belongs
  with the building, so the walks moved to the buildings layer and start where a terrain-layer way
  ends. Mounts gave layers a real question about who owns the join.
- *Order of shaping became a dependency.* A path that waits for a mount must be shaped before
  anything placed on that path. The first attempt broke every bookmark on a lane. Mounts, `on`,
  `along` and formulas are four ways of saying "this depends on that", each with its own order.
  They want one dependency graph.
- *The check is where the value is.* "No path reaches its door" is a fact about intent that no
  amount of geometry checking would find.
- *Moving the chapel also raised four placement errors.* The walk followed; the ground did not
  agree with where I put the building. That is correct, and shows a mount solves the join, not
  the placement.
- *A mount is too thin as built.* Position only. A door needs a direction so a path arrives
  square to it, a height so a stair can land on it, and a type so a road does not end at a window.

**Not done.** Mounts on members of sets, runs and kit runs; a kit piece's wall ends; stairs
landing on terraces; mounts read from named nodes in a model file; anything but a path using one.

## Three questions only doing could settle (2026-11-12)

### 1. What "the same thing" is after a rule changes

**Built.** Scatter members get ids (`woods:1234`, the number is the grid cell). Two ways to keep a
hand edit were built side by side: `[[override]]` patches a member by id; `[[pin]]` puts a thing
at a place and `[[clear]]` keeps the rule out of a circle. `[[lock]]` records what a rule placed
inside a region and replays the record. Every build writes `placed.json`; the next writes
`changes.json` against it.

**Measured** on `example`: three woods trees moved by (+6, +4), then the rule changed under them.

| Rule change | By name (override) | By place (pin and clear) |
|---|---|---|
| none | 3 of 3 | 3 of 3 |
| density to 60% | 2 of 3, 1 orphan | 3 of 3 |
| spacing 7 m to 6 m | 0 of 3, 3 orphans | 3 of 3 |
| seed changed | 2 of 3, 1 on a different tree | 3 of 3 |
| the wood retreats north | 2 of 3, 1 orphan | 3 of 3 |
| a new road through a moved tree | 3 of 3, silently on the road | 3 of 3, 1 `pin.conflict` |

Thinning with the pins in place reported `woods: +0 -98`, 247 untouched, no hand tree removed. A
lock round the north wood recorded 74 trees; under thinning it kept all 74, said `lock.holds
'woods' would now add 0 and drop 30 here`, and 70 trees went outside it.

**Lessons.**

- *A hand edit is intent at a place, not a patch to a produced thing.* No id scheme survived a
  change of spacing, because the things themselves are different things. The place does survive.
- *Names are still worth having, for the diff.* "98 removed, 247 untouched, here is the box" is
  what makes a rule change reviewable. That is their job; holding edits is not.
- *The by-name edit fails in the worst way once:* after a new seed it attached to a different tree
  and reported nothing. By place, the same case is impossible.
- *A lock is a recording.* It must say what the rule would now do differently, or it rots unseen.
- *Still open.* Edits to things that are not points (a path's vertex, a field painted by hand);
  a pin that should follow something ("this tree, beside that door"); what a lock does when the
  ground under it changes.

### 2. Placement by cause, with real assets

**Built.** World `glade`: one meadow, a house, a stream, a slope. The base scatters trees, bushes,
logs and stones evenly. Variant `by_cause` uses the same counts with `clump` (a parent and smaller
members round it), `around('trees', r)` (grow beside what an earlier scatter placed), bands along
the stream bank and the building edge, and stones by slope. Rendered both with a real asset pack.

**Seen.** Even placement strews boulders and logs over the open meadow and reads as noise. By
cause, trees stand in groups, stones and bushes gather on the bank, and the meadow is left open.
The difference is large and comes from four short expressions, not from better assets.

**Lessons.**

- *"Looks good unattended" is mostly "has a reason to be there".* Each rule that named a cause
  (beside a tree, on the bank, at the wall's foot) improved the picture; density tuning did not.
- *One scatter must be able to read another.* `around()` is the smallest form of that; it makes
  scatter order a dependency, which belongs in the one dependency graph.
- *This is judged by eye and there is no measure.* The verdict belongs to the person, per pack.
- *Not shown:* understorey by light, paths worn by use, variety within a clump by age.

### 3. An agent that did not write the tool

**Done.** A fresh agent got the documents and the example project, was forbidden the compiler
source, and was asked for a new world of its own: `monastery` (three layers, two tools, an
overlay, a winter variant; its log is `worlds/monastery/LOG.md`). 19 compiles, five rounds, errors
7, 1, 1, 0, 0; 8 of 8 reach expectations. It compiles unchanged here and is now in `check.py`.

**Verdict: yes.** The vocabulary sheet, the example and the kinds files were enough. What it hit:

- *Reachability was the main blocker.* The lint does not see decks, and `reach.not_sealed` gives
  no place to look.
- A formula in a run's `count` crashes with a traceback instead of a diagnostic.
- A generate step compiles the layers below it first: undocumented, and diagnostics show twice.
- `features[id]["points"]` in the SDK is every eighth sample, so path ends come up short.
- `explicit` rows lack `tint` and `scale`; the surfaces list in the vocabulary is incomplete.
- A passing lint step prints nothing. `bookmark.crowded` fires on a lamp that is not solid.
- It could not say: an enclosure with a gate, a stream that starts at a lake's outflow, "from A to
  B at no more than N percent".

**Lesson.** The diagnostics did the teaching: every round was driven by a message. Where a message
had no location or no code, the agent lost the most time. Caveat: one agent, one run, and the
model is the same family as the author's.

### 4. Text as the only source of truth

Cannot be spiked; only real users answer it. Result 1 narrows it: pins, clears and locks let a
person keep a decision without ever repairing the produced map.

## Two maps joined, the master owning only the bridge (2026-11-12)

**Asked:** build A and B apart; a master places both in open sea and owns only what connects them;
everything else at the joins blends by a strategy the user picks or defines.

**Built:** `worlds/strait/`. Two islands, each a world that compiles alone, and a master of 85
lines that is sea, two `[[part]]` rows and one bridge:

    starts_at = "a.quay_road.end"
    ends_at = "b.landing.start"

**What held.**

- *The master names places in its parts.* It could not before: the master's shapes were made
  before any part was compiled. Parts now compile first and hand over their path ends, points and
  instance mounts in the master's frame.
- *The bridge follows.* The overlay `changes/move_b.toml` is three lines and moves B by 40 m east
  and 24 m north. The bridge went from 250 m to 290 m and still lands on B's road. 0 errors.
- *A part sits in sea.* `expects = "sea"` skips the land check; both islands were lifted by 0.1 m
  or less because their own edges are sea bed.
- *The falloff is the user's.* `blend` on a part is an expression of the edge weight and the
  distance inside the edge. B uses its own; A uses the default.

**What it taught.**

- *A blend can silently drown what the master names.* With a 40 m apron B's road end stood in
  ground that was 37% sea bed, 1.5 m under water, and the bridge landed there without a word. A
  new warning, `part.named_in_apron`, now says so with the place and what to change.
- *The blend is blind to content.* It is a function of distance from a rectangle. It does not know
  a shore from a road. "Blend seamlessly" in the user's sense needs a strategy that reads both
  sides' fields, which is a step, and steps cannot yet run at the join.
- *Only ends can be named.* A master feature can start or end at a part's place. It cannot be
  placed `on` a part's road, and a part cannot be rotated to face the other.
- *A part's diagnostics are still in its own frame*, so they carry no position in the master.

**Lesson for the rewrite.** A part should publish named places as its interface, the way an asset
publishes mounts: the same idea at two sizes. The master then owns relations between published
places and never a coordinate inside a part.

## Every world in one map (2026-11-12)

Asked for fun and as a challenge: one master that places every practical world we have, joined
automatically so that it looks acceptable, to walk around in. World `atlas`: `city`, `college`,
`market`, `example`, `glade` (its `by_cause` variant), `relief`, `massif`, `vale` and `monastery`
as seam parts on a coastal plain, `station` hung on its door, and a tidal creek, one bridge and
eight roads that the master owns, every end given by a part's own road end. No part's file was
edited. 3,040 by 2,400 m at 1.25 m posts, 14,126 instances, 0 errors, 86 s (67 s of it the city).
`coast` is left out: it has parts of its own and parts do not nest.

**What had to change in the compiler** (about 110 lines, `parts.py` and one line of `pipeline.py`;
SPEC 12.6f):

| Obstacle | What was done |
|---|---|
| A part's steps did not run, so `city` arrived without streets | A part runs its generate steps and reuses their cache. One line. |
| Parts at 0.5 m and 1.25 m in one master | A part's rasters are sampled at the master's posts when the spacing differs. |
| The blend ate each world's own edge | `outer`: the part keeps all its ground, and the master's land comes to meet it outside. |
| Every world has its own zero (edges from 0 m to 140 m) | `lift`, said outright per part. Water is lifted with the ground. |
| Eleven ground materials, and the output holds eight | `ground_as` renames a part's material to one the master has. |
| `glade` is better in a variant | `variants` on a part. |

**What still looks wrong, by world**

| World | What is wrong in the atlas |
|---|---|
| city | Its river reaches the footprint's edge and stops; the skirt carries the bed out 110 m as a dry trough. The river itself drew no water in the renders (cause not found). Streets end at the edge on three sides. Asphalt is drawn as rock. |
| monastery | Its north edge stands 115 m above the plain. The skirt is a bare, smooth, straight-sided dome of rock. |
| vale | The same on its west edge: a straight scarp 400 m long. Its snow is drawn as rock. |
| massif | A straight scarp on the north and west; its lane climbs at 30% (its own warning). |
| relief, example, glade | Each sits on a low rectangular rise. Streams leave the edge and end. Sampled from 0.5 m to 1.25 m, so paths and banks are softer than in the world alone. |
| market | Good at the bridge. Its two north streets end in a field (only one is taken on). |
| college | Stands above the shore as it did in `coast`; its bookmark `bm_aerial` is reported as outside the extent, as it is in the world alone. |
| station | Stands on a levelled yard. Fine. |
| the plain | The master's roads inside a skirt follow the skirt, not their own grading, so they roll. The creek bridge climbs at its limit. |

**Lessons**

- **Meeting is a two-sided act.** A blend inside the footprint destroys what the part's author made
  near the edge; a skirt outside leaves the part whole and bends the master. Both are needed, and
  which one applies should follow from authority: the firmer side keeps its ground.
- **A shared datum is missing.** Sea level is the one number every world should agree on, and none
  declares it. A part should say what its zero means; `lift` by eye is the symptom.
- **A rectangle is the wrong boundary.** Every ugly thing above is a straight line. A part needs a
  footprint that is a shape (its ridge line, its shore), and declared things that cross it (a river
  leaves here at this level, a road leaves there) so the master can carry them on.
- **The skirt is still a blend of numbers.** It knows nothing about rock, slope or drainage. A
  master step at the join (erode, scatter, paint by slope) is where "a custom function" belongs.
- **Running a part's steps was one line.** It had been left out, not designed out.
- **Limits that only show at this size:** eight ground materials for a whole map, and no filtering
  when a fine part lands on a coarse master.

## Two maps joined by ground, the join in a layer of its own (2026-11-12)

Asked: build a place and the nature round it apart, then attend only to where they meet, which is
ground and not a bridge: growth across it, and the player funnelled through chosen points. World
`border`: `college`, unchanged, and a new 560 m `wild` (hills, a beck, woods by cause) that keeps a
corner clear. The master says where each stands and what the bare ground of the corner is. One layer,
`layers/join.toml`, owns the rest: a ridge, two openings, two walks, growth, three views and three
expectations. 0 errors, 4.7 s; reach 22 of 22.

| Question | Result |
|---|---|
| The join's line by name | Held. `path = ["wild.edge.2", "wild.edge.3"]`, `outset = 14`. A side has a number, which is the weak point: nothing says "the sides facing the college". |
| Openings as intent | Held. `open_at = ["south_walk", "west_walk"]` opens the ridge where each walk crosses it. Naming a place instead opened it at the nearest point, which was 18 m from where the walk passed. |
| An outline that is not a box | Held. The wild is an L. Its four outer sides lie on the world's edge and are still checked as if land met them, so `max_step` had to be raised for all six. |
| Growth across the join | Partly. `inside()` and `outside()` let a master field and scatter fade the wood over the bare ground (207 trees, 268 bushes). It is a second rule written to match the wild's, not the wild's own rule continuing. |
| A master feature on a part's ground | Held, by a new pass. Without it a 7 m ridge 12 m inside the wild's strip kept 4.1 m; laid after the parts it kept 7.0 m. |
| Against what a part anchored | Reported. The west walk moves the ground 1.3 m under a dormitory row's pad: `part.overruled`, a warning. The master wins. |
| The reach check across a join | Held. It reads the joined ground and the college's buildings. `closed` on an expectation shuts named openings: with both shut the wild is out of reach, with one shut it is not. Lowering the ridge to 1.5 m made the sealed expectation fail, as it should. |
| The college moved 30 m | Held with no edit. Both walks, both openings, the views and all 22 expectations followed. The south opening moved 20 m, not 30: it is where the walk crosses, and the walk now runs at a slant. The ridge stayed, since it is the wild's edge. |

What looks wrong in the renders (`out/border/`):

- The ridge is a smooth grey bank of one height. It reads as a rampart, not as land.
- The wild's outer sides are pressed down to the master's slope over the apron, a visible frame at
  the world's edge.
- Brush from the fringe rule stands on the ridge's flank: the rule did not read the field the ridge
  clears. The college's and the wild's own instances are never cleared by a master feature.
- The walk from the dormitory lane is a straight line between two names; it grazes a building's pad.

Lessons:

- **The master is a layer above its parts, and the spike had it below.** Master features were
  applied first and the parts blended over them. `after = "parts"` is a patch; the order itself is
  the finding.
- **A join is ordinary content plus three names:** a part's edge, a place where two things cross,
  and the opening that results. No join object was needed.
- **A funnel is proved by closing it.** "Reachable" alone says nothing about which way; "not
  reachable with these shut" does.
- **A master can name only what a part happened to name.** The west walk starts at a bookmark,
  because the college offers no road end on that side. A point along a path would have been the
  honest name.


## The edge of a world built apart: exits (2026-11-12)

The ask: in `atlas` the city's streets ended at its boundary out of nowhere. Fix it where it
belongs, and change the engine as needed.

**The split, as built.**

| Owner | What it does | Where |
|---|---|---|
| The city | A ring street 50 m inside the boundary. Every local street, and nine of the sixteen arterial ends, stop at it. The road step no longer closes districts with the map's edge, so nothing is laid out beyond the ring. Seven ways and the river's two ends carry on and are declared as exits. | `city/layers/arterials.toml`, `city/tools/network.py` (two lines) |
| The atlas | Answers all nine by name: three roads to neighbours, one highway off the map, three `fade`s, two stretches of river. No coordinate of the city appears. | `atlas/world.toml` |
| The compiler | `exit = true` on a kind; `part.exit` as a place with a facing; `onward`; the error `exit.unanswered`. | `parts.py`, `world.py`, `pipeline.py`, `diagnostics.py`, about 60 lines |

**What held.**

- The city alone still compiles clean, with the same twelve warnings as before. Exits mean nothing there.
- Removing one answer (`atlas/changes/forgot_north_road.toml`) stops the build with the exit's name,
  its facts and the line to write.
- A fade needs no coordinate: `starts_at` an exit, `onward = 150`. It follows if the city moves.
- The river's exits did what `atlas` had listed as a gap ("things declared to cross an edge"): the
  river now arrives from the north and reaches the sea.

**What it found.**

- **The river had been dry for a different reason.** The atlas exported no `water` field, so the
  viewer was handed none. One line. The earlier note "cause not found" is closed.
- **Answering the river broke two other checks, rightly.** Water at the city's edge tripped
  `expects = "land"`, so an exit now excuses the stretch of edge it takes up; that is the one
  attribute (`width`) the core reads. And the river's way in ran under the monastery's corner, which
  the build refused; the atlas moved the monastery 80 m and its road followed by name.
- **An answer is not checked.** A footpath may answer a motorway exit. The facts are quoted to the
  author and no further.
- **The contract runs one way.** A world says what leaves it. Nothing lets a parent say what it
  offers, so a world cannot ask "is there a river for me here".
- **A fade is one material.** It paints `path` from its first metre, so the city's asphalt changes
  to track at the boundary, not along the fade.
- **Exits stand 10 m past the boundary**, where the ways already ended, because a place on the
  boundary itself lies in the blended strip and is reported. The honest position is the boundary.

Lesson: the fix for "roads end nowhere" was not in the blend at all. It was a declaration on one
side and an obligation on the other. Most of the remaining ugliness at joins is likely the same
shape: something crosses an edge and neither side says so.

## A city shaped by the map it is placed in (2026-11-12)

**Asked.** In `atlas` the city still read as a square. Its road network should fade out naturally,
decided from the atlas's side, more like an open-world city. Is the premise wrong, that maps are
planned apart and then placed?

**Answer shown.** The premise is half wrong. A place with its own ground (an island, a valley) is a
map with an edge. A city is a rule that fills the land it is allowed. So a world may declare what it
can be given, with defaults, and its parent may give it (SPEC 12.6i): `[given.limits]` and
`[given.urban]` in `city`, `give = { limits = "city_limits", urban = "urban" }` and
`footprint = "city_limits"` in `atlas`. The part is then built in its parent's context. Alone, the
city builds as before: same 12 warnings, ring street and all.

| What | Before | After |
|---|---|---|
| City outline in `atlas` | The part's box, 1600 m square, closed by a ring street | A 36-point region drawn in `atlas`, along the coast, out along the roads, up the river |
| Local streets | Every one, to the ring | All where `urban` is high, every second further out, every fourth at the fringe, lanes at the last |
| Buildings | Zone by district | Zone by district, stepped down by `urban`: towers, mid-rise, rows, rows with gaps, none |
| Ring street | Always | Only when built alone; the road step lays it, the hand-drawn layer no longer does |
| Instances the city brings | 3,556 | 1,926 |
| Compile of `atlas` | 136 s | 184 s (the master is built once more, without parts, to read what is given) |
| Compiler | | about 95 lines: `parts.py` (`hand_down`, `take_given`, the exit corridor), `pipeline.py`, `steps.py` |

**What held.**

- One edit in `atlas` (the limits, or one number in `urban`) reshapes streets, lots and building
  heights in the city, with no edit to the city. Three shapes were tried in a row this way.
- The exits kept working without change: the way an exit stands on is carried past the limits to it.
- The plain's own grass and copses now come up to the ragged edge, because the part no longer owns
  its box.
- Steps needed no new call: a given region is a feature and a given field is a field in the build
  they are already handed.

**What did not.**

- **The arterials are drawn by hand in the city and are not given anything.** They stay straight,
  and one that has no exit simply stops at the limits. The grid behind them is what still reads
  as rectangular. Roads that follow the land or the limits are the city's rule to write, and it
  has not been written.
- **The parent draws the limits blind.** It cannot see the city's arterials or river when it
  writes the region (a given thing must exist before the part does), so "bulge along the roads"
  was done by looking at the map and typing coordinates. The city lifts `urban` along its own
  arterials to make up for it.
- **The city cannot reach the sea.** Its limits must lie inside its box, and the box sits 90 m
  inland. A waterfront needs the part's ground to give way to the master's shore, which is the
  reverse of what a seam does.
- **Every change to what is given rebuilds the whole part**, both steps, about 60 s. Nothing is
  rebuilt by area.
- **A seam check fired on the river.** Where the river crosses the limits the bed is a 7 m step;
  `max_step` was raised by hand. An exit should excuse the step as it excuses water.
- **One world of ours, one use.** Whether `given` is the right general mechanism (a forest given
  a climate, a village given a road to stand on) is not shown.

**Lesson.** "Built apart" and "built in context" are both needed, and the difference is a
declaration, not a second kind of world. What a parent may hand down has to be resolvable before
the part exists; that ordering rule is the real constraint, and it is the same one-graph question
as mounts, `on`, formulas and `around`.

## A city authored as intent at three strengths (2026-11-13)

Asked: if this were a city tool with authority and text first, what would it look like, and is that
a city feature or the core's? Built in `atlas` (the parent) and `city`; SPEC 12.6j.

**What the parent now writes for the city, and nothing else:**

| Strength | Written in `atlas` | Lines |
|---|---|---|
| Anchored | two squares (`market_square`, `cathedral_square`), four axes, the centre | 35 |
| Shaped | five districts, each a `where` and a character | 50 |
| Derived | `city_limits = where "sdist('kind:district') < 8"`; every street, lot and building | 3 |

The 36-point outline is deleted. A district's region is one line of intent, for example
`where = "dist('market_square') + 50 * fbm(140, 2, seed=31) < 270"`. Its character is `pattern`
(grid, organic, radial), `block`, `bearing`, `buildings` (taxonomy), `floors`, `density`.

**What held.**

- Three patterns read as three neighbourhoods on the map: bent streets round the market square, rings
  and spokes round the cathedral, a tower grid, sheds toward the sea, rows thinning outward.
- **The change case.** One overlay moves the market square 150 m west and 80 m north. The old town
  went with it; the limits, the seam street round it, the suburbs it uncovered and the lots all
  settled, with no other edit. Streets 129 to 123, city buildings 1,262 to 1,301.
- The city alone still builds (0 errors, 9 warnings, 111 s) from its own five default districts.
- A way with an exit is now carried only from the limits to that exit. Before, its far end ran on
  across open country to the old square's edge.
- Compiler cost: about 75 lines (`where` regions, `kind:` in `dist`, a set handed down, the carry).

**What did not.**

- **The skeleton is in the wrong place.** The main ways are the city's own, hand-drawn on a square
  grid, and a given thing cannot read a part. So the parent cannot say "along the grand road"; it
  restates four axes by coordinates, and a `city_room` rectangle for the ground the city's map
  covers. Anchored main ways belong to the parent, by where they go.
- Every district still hangs on the same north-south, east-west main ways, so from the air the city
  is still orthogonal between its neighbourhoods.
- The old town's buildings are 22 m boxes on 46 m blocks. The pack has no small urban piece.
- A district that comes to two pieces silently keeps the larger. Order of writing decides overlaps.
- The city cannot reach the sea: its own map stops 90 m short of the shore.
- The whole city rebuilds on any change: 200 s for `atlas`.
- The lot tool reads the committed pack file itself, because a step cannot ask for the pack.

**How far the three neutral mechanisms are needed, by trying.**

| Mechanism | Where the tools fake it | What could not be done as a result |
|---|---|---|
| (a) A network of paths and the faces between them as named regions | `network.py` calls `polygonize` on main ways, river, limits and district edges; faces become `w_<district>_<n>`, streets `st_<n>`, numbered in the order made | An overlay placing a thing `on = { feature = "st_40" }` fails: `'st_40' is neither a path nor a region`, because world text sits below the step. A layer above the step can name it, but changing one district's pattern renumbered a lot elsewhere from `lots_979` to `lots_965` |
| (b) Dividing a region into cells | `lots.py` walks each frontage and keeps rectangles that clear everything; a lot exists only as a kit run `lots_<n>` | `[[clear]]` over a generated building did nothing and said nothing: it speaks to scatters only. `[[pin]]` stood a well inside that building, no message. `[[override]]` by id was an orphan one build later. No `[[expect]]` can say "every lot touches a street", or name a block |
| (c) Inputs handed down | In the core since 12.6i; this turn added sets of features | Works. It is the only one of the three that is real |

So (a) and (b) are needed, and for the reason given: without them hand edits, names and checks stop
at the door of the two tools. (c) carried the whole demonstration. Also found: `[[override]]` with a
missing `id` crashes the compiler instead of reporting.

## Simulation under authority: stated facts, the Rust terrain model fills in the rest (2026-11-13)

Asked: start from a few authoritative statements (a lake exists, a river runs from A to B) and
let the existing erosion model work out the detail, instead of generating many terrains and
picking one. Shown in `worlds/basin`: 3.2 km square, the sea to the south, ground rising loosely
to the north (one formula), one lake at 72 m, one river from `spring` to `haven` with two rough
points between. The facts layer is four features. Everything else about the land is the model's.

**Route taken: the real Rust code**, not a stand-in. It built first time (13 s).

**What the Rust side accepted before.** Nothing from outside: `run(cfg)` made its own uplift and
starting surface from the seed, and its base level was the sea found by flooding in from the
border ring. Uplift and the starting surface were already described in the code as its only
outside conditions, and "base level" was already a per-cell mask, so the join was there to use.

**What was changed there.** 104 lines added, 6 changed, in three files; nothing reformatted.

- `via-terrain/src/stage.rs`: a `Given` struct (optional `initial_m`, `uplift_m_per_yr`,
  `held_m`), `run_given`, and two helpers. `run` now calls `run_given` with nothing given. A held
  cell is pinned after the border ring each step and added to the base-level mask.
- `via/src/main.rs`: `--initial`, `--uplift`, `--held`, each a raw `f32` raster.
- `via-terrain/src/lib.rs`: the two new names exported.
- `cargo test --release --workspace`: 16 tests pass, the determinism and gates-snapshot tests
  among them, so the default behaviour is unchanged.

**The step.** `tools/terrain.py` stands `after = "facts"`: it is handed that layer's build and
nothing above. It writes three rasters and a config, runs `via terrain`, reads back heights,
discharge, sediment, water depth and receivers, checks the facts, and writes `height`, `water`,
`wet`, `damp`, `silt` and the river's found course.

| | |
|---|---|
| Model grid | 288 x 288 cells at 12.5 m (the world plus a 16-cell margin), 160 steps of 1,000 years |
| Engine | 2.6 s |
| Step in all | 3.5 s |
| World compile, cold | 7.2 s (1281 x 1281 posts at 2.5 m, 36,621 instances) |
| World compile, a dress-layer edit | 3.6 s: "step terrain: reused, nothing it reads has changed" |
| Same text twice, from cold | identical `height.f32` and `world.json` |

**The finding that matters: a held cell is not how you state a river.** The plan was "anchored
features become held cells". Tried first, for both the lake and the river, and measured:

- River cells held at a falling level: the land beside the river was lower than the river along
  84% of its course. A held cell is base level, which means water ends there. It cannot pass
  water on, so the land next to it found its own way to the sea and cut below it.
- Lake cells held at a level: 69% of the shore ended below the lake, for the same reason.
- River as a plain groove in the starting surface, nothing held: the water left it after 48 m,
  because the line crosses a slope and the groove had no bank on the downhill side.
- Lake as a plain hollow, nothing held: it silted up (2 to 5 m of sediment) and its outlet was
  cut down 10 m; 8% of it still held water.

What holds:

- **A river is shape, not a held level.** A groove that only falls and lies below all ground
  within three cells of the line. The model sends its own water down it and deepens it. Levels
  along the river are then the model's, which is right: the author stated a course, not levels.
- **A lake is a hollow with a held shore.** The shore is the one place a held cell is correct: a
  sill the model cannot cut, where the streams that reach the lake end.
- **The edge of the map is held** at the starting surface, on a margin outside the world.

So authority maps onto two things in a model like this, not one: conditions it may not change
(held), and the surface it starts from (shape). Which one fits depends on whether water should
end at the thing or pass along it. A second simulation will have its own answer; the general
part is the contract: facts in, fields and found geometry out, facts checked after.

**Each fact is checked after the run**, with the model's own water, not with our assumptions.

- River: followed down the model's receivers from the source. It keeps to the stated line for
  2,730 m (the line is 2,646 m), ends 101 m from the haven, where it meets the sea, falls 123 m
  and gathers 21% of the land.
- Lake: fully flooded, standing at 72.0 m; the land falls away below it along 20% of its shore.
- A lint step checks the same facts again on the finished build, after the layers above.

**Change case 1: the mouth moves 500 m east** (`changes/move_haven.toml`, one point). The model
reran in 3.5 s. The lower valley and the tributary that joined it moved; the uplands and the lake
did not; the views placed `on` the river's course moved with it. 0 errors. The river keeps to its
line for 2,675 m and ends 156 m from the new haven.

**Change case 2: what the land cannot honour** (`changes/cannot_hold.toml`):

```
error terrain:stated.lake_dam tarn at (793.8, 1856.2): to hold water at 96.0 m, 96% of its shore
  would have to be raised by more than 3 m, and by 26.0 m at (794, 1856): the land falls away from
  it, so that is a dam, not a lake
  hint: a level of about 73 m would be held here; or move it into a hollow
error terrain:stated.uphill river at (2249.4, 2848.6): the land at its mouth (2249, 2849) is
  128.2 m and at its source (1500, 330) is -0.3 m: the river would have to run uphill by 128.5 m
  hint: swap its ends, or move the mouth to lower ground
```

Four more errors follow from the dress layer, whose views stand `on` a course that no longer
exists. They are true, and they are noise.

**Two ends only** (`changes/two_ends.toml`, `course = "find"`): works, 0 errors, gathers 29% of
the land. The found course is a walk on an eight-way lattice: dead south, then at 45 degrees.

**How it looks, honestly.**

- From the ground and at an angle it reads as land shaped by water: a wooded main valley, side
  valleys joining it, a lake in a hollow with streams arriving. Better than any hand-shaped
  ground in the spike, and nobody drew a valley.
- From above it does not: the drainage is rectilinear. Streams run along the grid's axes and turn
  at right angles, because the starting surface is a nearly uniform tilt and the cells are 12.5 m.
- The stated river is a smooth ribbon of constant width with stepped water edges; real rivers
  wander and widen. The model knows how much water each reach carries and the step ignores it.
- Tributaries are painted beds with no water in them.
- Relief is gentle: a few metres of valley over most of the map. More erosion gave bolder land
  and also erased the facts (the first try lowered the land by 34 m on average), so the amount is
  a dial with a narrow useful range.
- The join between the model's 12.5 m cells and the world's 2.5 m posts is cubic interpolation
  plus 0.45 m of noise. It does not show at this relief; it would on steep ground.

**What it says about the design.**

- The step contract carried a real outside simulation with no change to the compiler beyond one
  convenience (a point feature is a place by its id).
- The model's parameters were made for 200 m cells and uplift over millions of years. At 12.5 m
  with no uplift they needed retuning by trial; four numbers decide whether the facts survive.
- The found course is geometry the author did not write and other layers stand on. That is the
  same need as generated streets and blocks: derived things that must be nameable.

## A number for "boxy", and a town on simulated land (2026-11-13)

Two things, both town library work. The compiler changed by 8 lines (a point placed `on` a feature
can end a path); the Rust engine did not change.

### The measure

`kinds/tools/street_form.py`. Bearing spread: streets cut into straight pieces, bearings folded to
0..90 degrees, weighted by length, entropy of 18 bins over ln 18. 0 is one grid, 1 is every way
alike. "Order" is the orientation order of the street-network literature (Boeing 2019), 1 for a
perfect grid, given beside it. Junction shares count places where streets meet.

| Build | Streets | Spread, all | Spread, main ways | Order, all | Order, main | Four-way | Three-way | Dead ends |
|---|---|---|---|---|---|---|---|---|
| `city` alone | 160 | 0.45 | 0.40 | 0.82 | 0.92 | 49% | 41% | 10% |
| the city in `atlas` (districts) | 137 | 0.76 | 0.51 | 0.50 | 0.88 | 41% | 26% | 32% |
| `townland` | 110 | 0.87 | 0.97 | 0.27 | n/a | 42% | 19% | 39% |
| `townland`, hill moved | 110 | 0.80 | 0.99 | 0.23 | n/a | 41% | 25% | 34% |

The square city before `give`, and the limits-only version, cannot be rebuilt: nothing was
committed between them, and the tools were edited in place.

What the numbers say, and they agree with the eye:

- Districts did change the streets (0.45 to 0.76) but not the skeleton (0.40 to 0.51; order still
  0.88). That is "distinct neighbourhoods on an orthogonal frame", in two numbers.
- In `townland` the skeleton itself has no grid left (0.97), because nobody drew it.
- Dead ends tripled from the city alone to the other two (10% to 32% and 39%). Some are the fade at
  the edge; many are grid streets cut off where a ward ends. A real town has more three-way
  junctions than this. The measure shows a defect the eye had not named.
- The measure cannot see block shape, building size or the land. A town of curved streets and 22 m
  boxes scores well.

### Townland

1.6 km, 2 m posts, 52 s. The author states: a starting surface with two hills by name, a river by
its two ends, five gates, two crossings and a market placed on the river, a cathedral, six main
ways by `starts_at`, `ends_at` and `via`, and four districts as conditions. No street, bend or
block is written. The chain: terrain step (from `basin`, unchanged, 1.6 s) then skeleton step
(`kinds/tools/main_ways.py`, 1 s) then the city's street and lot tools.

| Main way | Found length | Straight line | Steepest over 40 m |
|---|---|---|---|
| high_street | 1,916 m | 1,624 m | 5.7% |
| north_road | 825 m | 720 m | 6.4% |
| strand | 1,688 m | 1,576 m | 5.6% |
| quay_road | 909 m | 566 m | 4.3% |
| church_way | 696 m | 204 m | 7.5% |
| back_lane | 727 m | 454 m | 5.2% |

What held:

- **The river bends because a hill is in its way,** not because someone drew a bend. The first
  layout had the hills to either side and the river came out dead straight: the search is only as
  interesting as the land it is given.
- **A grid that follows the nearest main way** (already the default in the street tool) is enough
  to stop districts sharing one bearing once the main ways curve.
- **The change case.** One overlay moves the hill 170 m. The river found another way round, the
  crossings and the market (placed on the river) moved with it, all six main ways were found again,
  and every street and lot settled. 0 errors.
- **A stated fact failed usefully once.** Tilting the land put the river's mouth 1.6 m above the
  sea; `stated.mouth_dry` said so with the position.

What did not:

- **Built beside `atlas`, not in it.** At 202 s a build, `atlas` was not practical to iterate in.
  The city world still has its own hand-drawn arterials and river, and `atlas` is unchanged.
- **166 warnings.** 163 are buildings on ground that varies more than they tolerate: the lot tool
  knows nothing of slope, and a lot does not level its ground. 11 are local streets steeper than
  12%: only main ways are routed over the land; local streets are laid as a pattern and ignore it.
- **Tributaries from the model are rectilinear** at this size and are hidden, not fixed.
- **The search has no notion of a junction.** Ways join because later ones are cheaper along
  earlier ones. Two ways can run side by side for a stretch before they merge.
- **A way's course is nobody's to edit.** There is no way to say "as found, but pass this side of
  the church" except another `via`.
- **Order matters.** Main ways are found in the order written.


## A town that holds up on foot: the river's edge, the bridge, the bank (2026-11-13)

`townland` was accepted from the air and failed at eye level. Five ground bookmarks were added
first (`bm_bank`, `bm_on_bridge`, `bm_by_bridge`, `bm_source`, `bm_mouth`) and shot before any
change; those pictures are kept in `out/townland/before/`.

### What the ground showed, and why

| Seen | Cause |
|---|---|
| Streets ended in grass short of the river, on both banks. | The street tool cut local streets to the faces between main ways and the river's edge. Nothing ran along the bank, so every street that reached it ended on nothing. |
| The water's edge was a staircase with hanging translucent sheets, and the water stepped down in terraces near the mouth. | The `water` field was set only in cells within 1.5 m of the channel and was -100 beyond. Where the bank was lower than the water, the water simply stopped at a cell edge. The viewer interpolates the field, so each edge became a vertical sheet. |
| The river began 30 m inside the map at full width, ran nearly straight, and met the sea with no mouth. | It was stated from a "spring" with one width, and its course was the model's cell-to-cell walk eased (sinuosity 1.05). |
| The bridge was a fan of stepped slabs. | The deck was cut out of whichever main way crossed. A way named `via = ["market", "old_bridge"]` reached the crossing point and left it in another direction, so the piece over the water doubled back on itself. The street tool then added its own deck where each arterial met the river, and a way that waded elsewhere got a third. Three decks lay on one another. |

### What changed

All in libraries. `worldc` is untouched.

- `worlds/basin/tools/terrain.py` (400 to 467 lines): inflow from an `exit`, width from source to mouth, an
  estuary, bends added after the model inside its valley, a stated cross-section with a lip above
  the water, the water's surface carried under the banks, and the water handed back as a region.
- `kinds/tools/main_ways.py` (192 to 289 lines): a crossing owns one straight deck and
  two raised bridgeheads; ways are routed to the bridgeheads; the deck is never cut from a way.
- `worlds/city/tools/network.py` (280 to 374 lines): `waterfront`, and loose ends joined or cut back
  (`join_within`, off unless asked for, so `city` is unchanged).
- `kinds/tools/street_form.py`: `dead_ends`, `stubs`, and a bridge counts as a street.
- `kinds/town.toml`: `waterfront`; `crossing` defaults. `worlds/basin/kinds.toml`: river keys.
  `kinds/ways.toml`: nothing grows under a `bridge`.

What the author writes now:

```toml
[[feature]]
id = "upstream"              # the river comes from beyond the map
kind = "exit"
class = "river"
at = [1200.0, 1600.0]

[[feature]]
id = "river"
kind = "stated_river"
starts_at = "upstream"
ends_at = "haven"
course = "find"
width = [9.0, 17.0]
estuary = 2.4
bends = 1.8
path = []

[[feature]]
id = "embankment"            # the river's banks are a frontage, where the town is
kind = "waterfront"
along = "river_course"
where = "(dist('river_course') < 70) & (sdist('kind:district') < 0)"
```

A crossing is written as before (`kind = "crossing"`, `on = { feature = "river_course", t = 0.52 }`).

### Numbers

| | Before | After |
|---|---|---|
| Dead ends | 39% (120 places) | 15% (35) |
| Stubs under 40 m | 28 | 9 |
| Four-way / three-way | 42% / 19% | 47% / 38% |
| Bearing spread, all / main ways | 0.87 / 0.97 | 0.83 / 0.94 |
| River sinuosity | 1.05 | 1.22 |
| Decks at the old bridge | 3, overlapping | 1, straight, 0.6 m end to end |
| Warnings | 166 (155 buildings on slopes) | 205 (193 buildings on slopes) |
| Compile | 52 s | 60 s |

The `move_tor` change case still settles with 0 errors: the river, its bends, both bridges, the
embankment and the streets all move with the hill. After it, dead ends are 16% (41, 7 stubs).

### What is still wrong

- Buildings on slopes went up, not down: the embankment is a new frontage on graded ground.
- Bends are a jittered sine. From above they read slightly regular.
- The deck is 36 m long over a 13 m river, and its parapet is stepped boxes.
- A district written as a condition was cut in two by the river's new banks and half the town
  disappeared without a message. The world now names the river's corridor in the condition. The
  silent drop is the defect (SPEC 12.8).
- The waterfront follows every bend of the river, so the embankment road wriggles.
- Tributaries are still not drawn.
- A tree can stand at the water's edge with its foot wet: growth reads the old notion of dry ground.
- The "before" map from the air was overwritten; the before pictures kept are the five ground views.
