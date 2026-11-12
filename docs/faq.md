# Questions a newcomer would ask

Written from the seat of someone deciding whether to adopt `via`. It is a way of questioning the
design, not a feature list: an answer here is not a promise. `via` is a placeholder name.

Every answer assumes an agentic workflow: an agent writes and edits the documents, runs the
compiler, reads its diagnostics and looks at rendered views; a person states what they want and
reviews. "You write" below means you and your agent.

Every answer also assumes the asker uses Godot or Unreal. Other engines can come later and are
not weighed here.

Each answer carries one mark:

- **Shown**: it works in the spike, in a world you can compile. The world is named.
- **Decided**: we have chosen, and nothing is built.
- **Open**: we have no answer yet. The answer is left blank on purpose.
- **Not ours**: we do not intend to own this. The answer says who does.

Count at the time of writing, of 126 questions: 65 shown (27 of them only partly), 4 decided,
44 open, 13 not ours.

## Is it for me?

**What is it, in one sentence?**
Decided, as the goal. You decide what matters and the world settles around it; change your mind
and it settles again; you never repair a map by hand.

**And what does that mean in practice?**
Shown, partly. You state the decisions you care about (this stands here, that runs there) and
they hold. What you did not decide is worked out to fit: ground under a building, growth beside a
road, the seam where two things meet. A change is an edit to a decision, not a repair. Holding
decisions is shown, and so is following a change. That the undecided rest *looks good* is not:
our worlds are stand-in shapes. (`example`, `vale`)

**Do you generate assets?**
Not ours. We assume you make them quickly yourself, with generative tools or otherwise, and keep
replacing them. That is why the tool is about placing them well at scale, repeatedly, and not
about making them.

**My assets are generated, so their size, pivot and facing are all over the place. Is that a problem?**
Open. Size is measured from the file. Facing and pivot are assumed, and a kit run wants pieces of
one exact width, which generated pieces will not have.

**And at a glance?**
Shown. A compiler for game worlds. In: text files describing terrain, water, roads, buildings and
dressing by intent, plus a pack binding asset names to your files. Out: a folder of terrain
rasters, every placed object and generated meshes for an engine to load, and a list of what does
not fit. Change a line, rebuild, and what depended on it follows.

**Who is it for?**
Decided.
- *New games.* A world that starts here, more than a world brought from elsewhere. The developer
  may be experienced; it is the game that is new.
- *Independent developers first.* They take up a new tool more readily and have fewer people
  managing assets.
- *Large studios too, later.* Adoption is harder for them, not unwanted. The engineering keeps what
  they need: reproducible builds, headless runs, budgets, layers for teams.
- *Godot or Unreal*, and an agent doing the writing.
Not for: someone who wants an editor window, 2D or tile games, or worlds generated while the game
runs.

**I am one person with no tools programmer. Can I use it?**
Open. The agent stands in for the tools programmer; every world here was made that way. Whether a
person who cannot read the documents can steer one well enough is not known.

**Is it a game engine?**
Not ours. It produces files an engine loads. Rendering, physics, navigation and gameplay stay
with the engine.

**Is it a level editor? Is there a window where I drag things?**
Not ours. There is no editor. You or an agent write text; you look at the result in your engine.

**Does it generate a world for me from a prompt?**
Not ours. It is not a generator. An agent can write the documents, and the compiler checks them.

**What is the smallest thing I can make with it?**
Shown. A height field, one road and one house: about 20 lines. (`docs/design/blackbox.md`)

**What is the largest thing anyone has made?**
Shown. A 1.6 km city with 159 streets and about 3,900 buildings, compiled in 70 seconds. Real
open-world sizes have not been tried. (`city`)

**Can it build a map like a big-budget open-world game's?**
Not ours, mostly. Such a map is structure, hand-made content and a streaming runtime. We aim only
at structure: roads, districts, terrain, placement. Shown for that at 1.6 km, with boxes. (`city`)

**How big a world can it handle?**
Open. There is no tiling, so tens of kilometres would not compile.

**Is it ready for production?**
Shown, as a no. Everything so far is a throwaway spike in Python. The real thing is not started.

## Writing a world

**What do I actually write?**
Shown. TOML files: fields (a value at every point), features (points, paths, regions), kinds
(what a feature does to fields), instances and rules that place them. (`example`)

**Why text and not a painted map?**
Shown. So that moving a road is one edited line and everything that depended on it follows.
Seven such changes were run on one world. (`vale`)

**Do I have to learn a language?**
Decided. To read, not to write: the agent writes it. TOML for structure, plus small expressions inside strings, such as
`"1 - smoothstep(0, 8, dist('lane'))"`. The expression language itself is not yet defined.

**Can I place one exact thing by hand?**
Shown. `[[instance]]` with `at` and `yaw`, or `on` another feature so it follows that feature.

**Can I sculpt terrain by hand?**
Shown, partly. You can bring a heightmap picture and use it as a field. There is no brush.
(`relief`)

**I already have a heightmap, a splat map or a noise texture. Can I use them?**
Shown. A field can read a picture, by channel, with a range and a place in the world. (`relief`)

**Can I use my own math for terrain?**
Shown. A script handed the world so far returns fields and features. One world's land is 99 lines
of the user's Python. (`massif`)

**How do I see what I made?**
Shown, for Godot. A viewer loads a build and renders named views; you can also walk it.

**How do I know something is wrong?**
Shown. The compiler reports coded diagnostics that name the thing and the place, for example a
road steeper than its limit or two buildings claiming different ground. There are 88 codes.

**Can it tell me my world is boring or makes no sense?**
Open. A uniform city grid compiled without one complaint.

## Assets

**Does it come with assets?**
Not ours. It ships stand-in shapes so any world builds. Models and textures are yours.

**How do I connect my models?**
Shown. A pack binds each asset ID to a file and states facts about it. The world never names a
file, so swapping the pack re-dresses the same world. (`vale` with two packs)

**What must my models satisfy? Which way is front? Where is the pivot?**
Open.

**I have a modular building kit. Will it snap?**
Shown, partly. Pieces of one width repeat along a line or a street. Corners, end pieces of other
widths and filler pieces are not handled. (`market`, `example`)

**Can I organise and search my assets?**
Shown, as a first cut. Assets can be filed in a tree with labels, browsed, filtered by any fact,
and selected from a world by role instead of by name.

**Can I list the assets a world uses, and find the ones nothing uses?**
Shown. Use is read from what was built, so anything placed by a rule or a scatter counts. Over
six test worlds: 97 of 116 assets placed somewhere, 19 placed nowhere, with a count per asset and
the worlds it appears in. Open: the reverse case, a world that names an asset the pack lost, is
an error at build time but is not listed across worlds.

**My trees and rocks are made by a script in my engine, not files. Does that work?**
Shown, for one case. A pack entry can name a generator that the engine side runs.

**Will it convert, optimise or make lower-detail copies of my models?**
Not ours.

**Will it store or version my assets?**
Not ours. They stay where you keep them.

## My engine

**Does it work with Godot?**
Shown, as a spike viewer. A proper loader is not written.

**Does it work with Unreal?**
Open.

**Unity? My own engine?**
Not ours for now. Godot and Unreal are the two engines considered; others can follow.

**What does my engine receive?**
Shown. A folder: rasters for fields, every placed instance with its transform, generated meshes,
and a manifest. Rules never reach the engine; only their results do.

**Will it run at a good frame rate?**
Not ours, with one duty. We hand over instances grouped so an engine can batch them. One world
went from 12 to 114 frames a second from that alone. We promise no number.

**Is level of detail a concept here? Does the map define it, or the asset?**
Open, with a position. Not a concept today: a pack entry names one model file and says nothing
about lower-detail copies. Position: the copies belong to the asset, the switching distances to
the engine, and the map defines neither. What the map can rightly say is importance: a landmark
that should stay visible from afar. What we would carry: the asset's set of copies in the pack,
handed through untouched; and per-chunk counts of instances and triangles, which exist now.

**Does it stream a large world in and out?**
Open.

**Does it make collision and navigation meshes?**
Open.

**Can it rebuild the world while the game is running?**
Open.

## Changing things later

**I moved a road. What happens to the houses along it?**
Shown. Things placed `on` or `along` the road move with it. Things placed by coordinates stay,
and the compiler says if they now conflict.

**I swapped a building for a bigger one.**
Shown. The ground under it is re-levelled, and overlaps with neighbours are reported.

**An artist changed a kit piece's size. What breaks?**
Open.

**I generated a forest, then moved three trees by hand. If I regenerate, do I lose that?**
Open.

**Can I have a winter version, or a damaged version, of the same map?**
Shown. A variant is a named change merged over the world. (`vale`, `example`)

**Can I try a change without touching the real files?**
Shown. An overlay is a separate file applied at build time. (`example`)

## Working as a team

**Can several people work on one world?**
Shown. Layers are separate files with owners. A layer may refer to lower layers, never higher.
(`college`)

**My game is just starting. Can I build the map in pieces and join them later?**
Shown, partly. Each piece is its own world, built and walked alone. A master map then places it
in one of two ways (`coast`):
- *At a seam:* the piece brings its terrain, blended into the master's over a margin you set.
  The compiler refuses if the two grounds are too far apart, or if a piece that expects land
  meets water.
- *Through a door:* an interior hangs on a door of a building in the master.
A piece says where its roads leave, so the master's can meet them. Known limits: a piece can be
moved but not turned; pieces do not nest; all pieces together share eight ground materials; a
piece's own water level does not move with it.
Open: loading pieces separately while the game runs.

**Does it merge cleanly in version control?**
Open.

**How do we review a change to a map?**
Shown, partly. Named views are re-rendered and diagnostics compared. Nothing shows a map diff.

## My own code

**I need something you do not have. Am I stuck?**
Shown. You can define new kinds as data, add fields, attach your own data to anything, and join
the build with your own script. (`example`, `city`)

**In what language?**
Decided. Any, through a file contract. A Python helper exists; Python is the intended SDK.

**My terrain comes from my own procedural logic (say, a sea floor). Can I bind it, and is `via` still useful?**
Shown for the binding. Your script joins the build as a step and hands back the height and any
fields of your own (depth, current, kelp). (`massif`) After that `via` does what it does on any
land: places wrecks and reefs, scatters by your fields, levels ground under what you anchor,
keeps layers, variants and checks.
Less useful when: the terrain is generated while the game runs; or the experience is mostly in
the water column, since the model is a ground surface with things on it. Open for under water:
movement checks for a swimmer (the reach check walks), and overhangs and caves beyond one per
column. Light, fog and the water itself are the engine's.

**Can my code run in the middle of the build, not only before and after?**
Open.

**Can I write my own checks?**
Shown. A lint step reads the finished build and reports in the same form as ours. (`example`)

**Can my quest, spawn and loot data live in the map?**
Shown. Any extra attribute passes through to the build untouched, and you can define kinds the
compiler knows nothing about. What it means is yours.

**Will the same input always give the same output?**
Decided, with a limit. For our part, yes. For what your script produces, that is yours.

## Specific things

**Can I lay out the player's path first and build the world around it?**
Shown. A path is a feature. Terrain can be shaped along it, things placed at a fraction of its
length and an offset, growth thinned by distance from it, and all of it follows when the path
moves. (`example`, `vale`)

**Can it help make each stretch of that path interesting?**
Shown, partly. It can check what you state: a place is reachable on foot, a view of a landmark is
not hidden by a hill, a climb is not too steep. Open: beats along a path (reveal here, fight
there, rest after), sightlines along its whole length, and pacing. What is interesting is yours.

**Vegetation: how much control do I have without placing each tree?**
Shown, partly. You control where and how thick through fields, never tree by tree:
- *A density field* says where forest is, as an expression: wetter ground, below the treeline,
  off steep slopes, away from a path. Change the field and the woods move. (`vale`, `example`)
- *A scatter* turns a field into trees: spacing, a weighted list of assets, a size range, a seed.
- *An outline* fences it: dense inside a region reads as an orchard.
- *A picture* can be the field: paint a mask.
- *What you build clears it by itself:* roads and platforms empty every field tagged vegetation,
  and an asset can declare fields it avoids.
- *Your own script* can supply a field, such as wetness from distance to water.
Open: clumps. A scatter is a jittered grid thinned by density, so clumping comes only from noise
you put in the field. Also open: undergrowth that follows the trees, a reusable "biome" that
bundles fields and scatters, and keeping a hand-moved tree when the scatter runs again.

**Rivers and lakes?**
Shown. A lake can be given a shape, or left to fill the ground and find its own shore. A flowing
river along a slope is not handled.

**Caves and overhangs?**
Shown, partly. (`coast`)

**Building interiors?**
Shown, partly. Rooms on levels, doors, stairs, furniture down to small props. (`station`)

**How fine is the control indoors? A ceiling fan, a picture on a wall?**
Shown, partly. Any object can be put at an exact spot in a room, against a wall, on top of
another object, or at a height above the floor (`mount`), which is how a picture, a shelf or a fan
is hung; the compiler refuses one hung across a door or window. Groups (a table with its chairs
and plates) are placed as one. One object can be marked as the interactive one among many alike.
Open: nothing checks clearance, such as a chair that cannot be pulled out. That the fan turns is
the engine's.

**Can I say "under the ceiling" instead of working out a height?**
Shown. `mount = "ceiling"` hangs a thing so its top meets the ceiling of the room it is in, with an
optional `drop`. Raise the room and the fan follows. (`station`) The same idea already holds for
`on` a path, `atop` an object, `beside` one, and against a wall. Open: other surfaces by name
(a wall's face at eye height, a beam, the underside of a stair).

**Fine control makes files long. Can I split and reuse definitions?**
Shown, partly.
- *A set:* a table with its chairs and plates, written once, placed many times.
- *A rule in place of a list:* a run along a wall, things scattered on a shelf.
- *Layers:* shell, furniture and small props as three files with three owners.
- *Include:* any file of definitions, shared between worlds.
- *A part:* a whole interior as its own world, hung on a door of the building outside.
Open: a set cannot take a parameter or hold another set; a layer cannot be limited to one room;
and nothing collapses detail you are not looking at.

**When it builds a city, is that rule-based? How does it work?**
Shown. Yes, in two tiers: you draw the top level and rules derive the rest. (`city`)
1. *By hand, 39 rows:* a river, arterials, freeways, a point per district saying its character
   and grid angle, a point per interchange.
2. *A road rule:* districts are the areas between arterials; each gets its own grid of local
   streets; a bridge where an arterial meets the river; ramps at each interchange.
3. *A lot rule:* walk every street front and keep a lot wherever it is clear; the district
   decides tower, mid-rise, row house or shed.
4. *The compiler:* grades the streets, places every building, checks clearances and overlaps.
The rules are the project's own scripts (about 360 lines), not built in. What they hand back is
plain streets and rows you can read and override.

**Roads, bridges, overpasses, ramps?**
Shown, partly. No junction geometry. (`city`)

**A minimap or world map for the player?**
Shown, partly. The map is emitted as data layers; the painted look is yours. (`coast`)

**Indoors, can I have a map per floor?**
Shown, partly. An interior is written as rooms on levels, and the compiler draws one plan per
level: rooms, walls, doors, stairs. (`station`) That is a picture for review. Open: the same as
map data a game can restyle, switching floor as the player climbs, and rooms revealed as visited.

**Can it check that the player can actually get somewhere?**
Shown, partly. You state who walks and where they must reach. It reads the ground only.

**Does it support weather?**
Not ours, for weather that happens: rain, falling snow, wind, changing light are the engine's.
What we can give it: a world in another state, as a variant (a winter map with snow on the ground
and bare fields, `vale`, `example`); and fields a weather system can read, such as wetness or
where snow would lie. A `look` table passes sun and sky hints through; who owns it is open.

## Using it with an AI agent

**Is it made for agents?**
Shown. Every world here was written by one, working from diagnostics and rendered views.

**Can an agent work from a concept image?**
Open.

**What stops an agent from making a mess?**
Open.

## Adopting it

**I already have a game. Can you import it?**
Shown once, as a port, not an import. Not a priority: the tool is aimed at games that start here. There is no importer; you write a one-time converter, and
how far it gets depends on where your map lives:
- *In data files of your own* (positions, shapes, rules): convert them to our documents. Done for
  one project in about 300 lines: terrain within 25 cm on 97% of land, every object within a metre.
- *In an engine scene, placed by hand*: export the heightmap and every transform. Open: nobody has
  tried it.
- *Assets*: they stay where they are. You write a pack that names them.
- *What your game generates at run time* (trees, walls, roofs): stays in your runtime.

**Do I get my design back, or only its result?**
Shown. The result first: a heightmap and a list of placed things, exact but baked. The intent
("eighteen trees in this glade") has to be rewritten rule by rule, and lands near, not on, what
you had.

**What do I have to give up?**
Open.

**What does it cost? What is the licence?**
Open.

**Who else uses it?**
Open.

**What happens to my worlds if you stop?**
Open.

## Asked by two outside readers

Two fresh readers were given one paragraph about the tool and a role, and nothing else: S, a senior
developer at a large studio (who assumed a proprietary engine; read those questions as Unreal); B, an independent developer with no tools
programmer. Their questions, where the sections above did not already answer them. A blank answer
means we have not faced the question.

| | Question | Mark | Answer |
|---|---|---|---|
| S | Is this meant to replace our world editor, sit in front of it, or run beside it? | Open | Today the text is the only source of truth and nothing reads an editor's changes back. |
| B | Someone moves a rock by hand in the engine editor. What happens on the next rebuild? | Open | It is lost. There is no way yet to keep an edit made outside the text. |
| S | How does a level artist nudge a rock 30 cm in a text file? | Open | By asking the agent, or editing one number. Whether that is acceptable to an artist is untested. |
| B | Can I lock an area as finished while the rest keeps regenerating? | Open |  |
| B | I change one line. How do I see what changed before I accept it? | Open | You get new diagnostics and re-rendered views. Nothing lists which objects moved. |
| S | I upgrade the tool mid-project. Does my shipped world move? | Open |  |
| B | Does it work without an AI agent? | Shown | Yes. The inputs are plain text files and the compiler contains no model. The agent is a convenience for writing them. |
| S | Is there AI inside the build itself? | Shown | No. The build is a deterministic program. A model was only ever used to write documents, and once to restyle a map picture. |
| S | Where does the agent run, and what does it see? | Not ours | You choose and host the agent. The compiler runs locally and sends nothing anywhere. |
| S | Is anything in the output generated by a model? | Shown | No. The output is your assets arranged, plus terrain and meshes computed from your text. |
| B | Do I have to disclose AI on a store page? | Not ours | That is between you, your agent and the store. Nothing in the build is model-made. |
| S | Who is accountable for the content? | Not ours | Whoever reviews and ships it. |
| S | A ten-thousand-line diff tells nobody whether a town looks right. What does a reviewer look at? | Shown, partly | Named views re-rendered from the same spots, and the list of diagnostics. There is no visual diff. |
| S | What catches a road that reads fine in text and cannot be driven? | Shown, partly | Checks you can state: grade limits, clearance under a bridge, reach on foot. Nothing checks driving. |
| S | How long from changing a line to seeing it? Is the rebuild incremental? | Shown, partly | Seconds for a village, 70 seconds for a 1.6 km city. It rebuilds the whole world; only user scripts are cached. |
| S | Forty people on one world. How do merges work? | Open | Layers split the files by owner. Merging and two people changing things that depend on each other are untested. |
| S | Does it run headless on a build farm? Can builds be cached and shared? | Shown, partly | It is a command with an exit status, so headless works. There is no shared cache. |
| S | Does it know my budgets: memory per cell, draw calls, certification limits? | Shown, partly | It counts instances and triangles per chunk against limits you set. Memory, draw calls and certification are the engine's. |
| S | Fit by whose rules? Can I add rules, and can a failure block CI? | Shown | Its own seam and placement checks plus yours, written as lint scripts. Any error gives a non-zero exit. |
| S | What does it do that Houdini and our in-house tools do not? | Open | A claim, not evidence: the whole world as reviewable text, checks that name the cause, and an agent loop. It is not a replacement for a generator. |
| S | Does the pack understand our asset database, variants, LOD chains, material instances? | Open | Today it is a flat table of one file per name, kept by hand. |
| S | Does the output carry collision, navmesh inputs, streaming cells, audio zones, gameplay markup? | Shown, partly | Chunk grouping, solidity, your own markup passed through, and any field you export. The rest is not produced. |
| S | Can it produce a hero location designed on purpose, or only filler between them? | Shown, partly | Exact placement and landmarks the terrain gives way to. Sightlines, cover and pacing are not concepts. |
| S | Show me a world built with it beside one built by an environment team. | Open | There is none. |
| S | What has shipped with it? | Shown | Nothing. It is a throwaway prototype. |
| B | Who makes it, how many people, will it exist next year, do I get the source? | Open |  |
| B | If I drop it after a year, what do I keep? | Shown | The build folder is plain rasters, JSON and mesh buffers, and the documents are TOML. Nothing is locked in a private format. |
| S | What does it cost me to find out it does not work for us? | Open | A loader for Godot or Unreal has to exist first. Today there is a prototype Godot viewer and nothing for Unreal. |
| B | Is there a ready importer for my engine? If not, how hard is it to write? | Open | It should be ours to supply for Godot and Unreal. Only a prototype Godot viewer exists. |
| B | How much work is it to bind a bought pack of a few hundred meshes? | Open | One short entry per mesh by hand today; sizes are measured from the file. Nothing generates a pack from a folder. |
| B | What machine do I need? Windows, Mac, Linux? | Open | Only tried on a Mac. |
| B | Is there a complete example I can open in ten minutes? | Shown, partly | There is an example village and a viewer, but nothing is packaged for download. |
| B | What are the generated meshes, and are they game-ready? | Shown, partly | Bridge decks, walls, floors, stairs and cave rock. Flat shaded, no texture coordinates: stand-ins. |
| B | Will everything look samey? | Open | It looks like the assets and the rules you give it. Our own worlds are grey boxes. |
| B | Can my quests and triggers hang on to places and survive a rebuild? | Shown, partly | Anything you name keeps its name. Scattered things have no stable identity. |
| B | Which AI does it need, and what does a day of work cost? | Not ours | Any agent that can edit files and run a command. Its cost is yours. |
| B | How long to learn enough of the format to review what the agent wrote? | Open |  |
| B | Who owns the world I build, and can I ship it commercially? | Open | No licence is chosen. |
| B | Where is the honest list of what it cannot do? | Shown | `spike/SPEC.md` section 11, `docs/challenges.md`, and every Open in this file. |
| B | Where do I go when I am stuck on a Saturday night? | Open |  |
| B | How many hours from download to walking around in something of my own? | Open |  |
