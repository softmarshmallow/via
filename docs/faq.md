# Questions a newcomer would ask

Written from the seat of someone deciding whether to adopt `via`. It is a way of questioning the
design, not a feature list: an answer here is not a promise. `via` is a placeholder name. What we do
promise, and what we do not, is in [`charter.md`](charter.md).

Every answer assumes an agentic workflow: an agent writes and edits the documents, runs the
compiler, reads its diagnostics and looks at rendered views; a person states what they want and
reviews. "You write" below means you and your agent.

Every answer also assumes the asker uses Godot or Unreal. Other engines can come later and are
not weighed here.

Each answer carries one mark:

- **Shown**: it works in a world under `examples/` that the engine builds. The world is named.
  (Until M6 this meant the spike.)
- **Decided**: we have chosen, and nothing is built. The answer cites the ruling or the chapter.
- **Open**: we have no answer yet. The answer says who gives one: the owner, or real users (people
  outside the project building with it).
- **Not ours**: we do not intend to own this. The answer says who does.

On 2026-11-17 (M7) every question then open was answered from a ruling (R numbers are in
`docs/doctrine.md`), a spec chapter or an example's numbers, or kept open with who answers it;
answers that had gone stale since the spike were brought to the rewrite.

Count on 2026-11-17, of 125 questions: 81 shown (33 of them only partly), 16 decided, 11 open
(each naming who answers it), 17 not ours. On 2026-11-13 it was 67, 9, 34 and 15.

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
Shown, partly. Size, bounds and triangles are measured from the file (`spec/assets.md` 3); facing
and up are named once per pack or asset (`front`, `up`, glTF's by default) and the pivot is a rule
(`base_centre` by default, 2.3 and 2.4), so an odd export is one key, not a re-export. A kit run
fills what whole pieces leave with fillers and caps its ends (`lib/town/kit.py`, `market`). Open,
for real users: whether one key per asset is cheap enough when models arrive by the hundred.

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
Open, for real users. The agent stands in for the tools programmer; every world here was made that
way, and at the M7 close agents of two model families (both Claude models, Fable and Sonnet;
another vendor's is the owner's to run) each built their world from the documents alone. Whether a
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
Shown. Thirteen worlds joined on one 3.2 by 2.56 km map, among them a 1.6 km city, with roads that
answer every exit; a cold build takes four to six minutes on one laptop (`atlas`: release `via` on
an M4 Pro, one run each with other builds beside it, 245 s to 325 s; protocol in
`examples/atlas/expect.toml`). Real open-world sizes have not been tried.

**Can it build a map like a big-budget open-world game's?**
Not ours, mostly. Such a map is structure, hand-made content and a streaming runtime. We aim only
at structure: roads, districts, terrain, placement. Shown for that at 1.6 km, with boxes. (`city`)

**How big a world can it handle?**
Shown to a few kilometres: `atlas` is 3.2 by 2.56 km, cut into chunks (128 m a side there, `spec/build.md` 5).
Tens of kilometres are set aside until after the first cut (doctrine section 7): fields are
evaluated by window, so tiled storage can come later without changing results.

**Is it ready for production?**
Shown, as a no. The engine is a rewrite in Rust (R19) at the close of milestone M6 of
`docs/plan.md`; M7 puts it before outside agents. Nothing has shipped with it.

## Writing a world

**What do I actually write?**
Shown. TOML files: fields (a value at every point), features (points, paths, regions), kinds
(what a feature does to fields), instances and rules that place them. (`example`)

**Why text and not a painted map?**
Shown. So that moving a road is one edited line and everything that depended on it follows.
Seven such changes were run on one world. (`vale`)

**Do I have to learn a language?**
Shown. To read, not to write: the agent writes it. TOML for structure, plus small expressions inside
strings, such as `"1 - smoothstep(0, 8, dist('lane'))"`: a small language of our own with a closed
list of functions (R6, R28), defined in `spec/formulas.md` with its list in `spec/functions.toml`.
(`ford`)

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
road steeper than its limit or two buildings claiming different ground. There are about 360
codes, each with what to change (`spec/diagnostics.toml`, `docs/reference/diagnostics.md`).

**Can it tell me my world is boring or makes no sense?**
Shown, partly. A uniform city grid compiled without one complaint. A street-form measure now
reports how boxy a town is beside the build, and the eye still decides (R15, `townland`).

## Assets

**Does it come with assets?**
Not ours. It ships stand-in shapes so any world builds. Models and textures are yours.

**How do I connect my models?**
Shown. A pack binds each asset ID to a file and states facts about it. The world never names a
file, so swapping the pack re-dresses the same world. (`vale` with two packs)

**What must my models satisfy? Which way is front? Where is the pivot?**
Decided, and shown on the packs of `examples/`. A glTF file per asset (`spec/assets.md` 3.1). Front
and up are glTF's (+Z, +Y) unless the pack says otherwise (2.3); the pivot is a rule, the base's
centre by default (2.4); size, bounds and triangles are measured, never written (2.2). A
lower-detail copy exported with another origin is warned (`asset.detail_pivot`, 3.3).

**I have a modular building kit. Will it snap?**
Shown, partly. A kit run lays bays along a path with corner, end and filler pieces, and leaves gaps
where a street crosses (`lib/town/kit.py`, `market`). Open: a real, non-box kit (challenge 33).

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
Shown. A loader addon turns a build into a scene, streams it by chunk and draws the player map;
a sheet tool renders and walks the named views (`engines/godot`, Godot 4.7).

**Does it work with Unreal?**
Decided: not in the first cut (R27, doctrine section 7). Godot comes first; the build directory
keeps an Unreal mapping on paper.

**Unity? My own engine?**
Not ours for now. Godot and Unreal are the two engines considered; others can follow.

**What does my engine receive?**
Shown. A folder: rasters for fields, every placed instance with its transform, generated meshes,
and a manifest. Rules never reach the engine; only their results do.

**Will it run at a good frame rate?**
Not ours, with one duty. We hand over instances grouped so an engine can batch them. One world
went from 12 to 114 frames a second from that alone. We promise no number.

**Is level of detail a concept here? Does the map define it, or the asset?**
Shown, partly, as the asset's. A pack entry may list lower-detail copies (`detail`), measured and
checked to share the main model's pivot, and handed through (`spec/assets.md` 3.3); the switching
distances are the engine's (principle 12), and the map defines neither. Per-chunk counts of
instances and triangles feed budgets (R44). Open, for the owner: whether the map may mark
importance (a landmark that should stay visible from afar).

**Does it stream a large world in and out?**
Shown, in our Godot loader. It loads chunks near the walker, keeps those whose `digest` is unchanged
on a rebuild, and draws a far layer past the load radius; a streamed walk ends as the whole build's
does (R48; `atlas`, 126 views). Streaming in your own game is the engine's (principle 12).

**Does it make collision and navigation meshes?**
Shown, partly, for collision. The loader gives the terrain, floors, decks and generated meshes
bodies, and hands on each asset's declared `collision` (R35). Navigation meshes: not ours; the
engine bakes them from what it loads (principle 12). Reach on foot is checked by a walk library
(R41).

**Can it rebuild the world while the game is running?**
Not ours: worlds generated while the game runs are outside the intended use (`docs/charter.md`).
What we give: a new build a reader can take chunk by chunk, keeping what did not change (R48).

## Changing things later

**I moved a road. What happens to the houses along it?**
Shown. Things placed `on` or `along` the road move with it. Things placed by coordinates stay,
and the compiler says if they now conflict.

**I swapped a building for a bigger one.**
Shown. The ground under it is re-levelled, and overlaps with neighbours are reported.

**An artist changed a kit piece's size. What breaks?**
Shown. Nothing the author wrote changes: the new size is measured, what follows from it settles
and is reported (R30), and an edit to an accepted pack is `asset.revised` (`spec/assets.md` 7).
`market` runs a remodelled piece and a changed module.

**I generated a forest, then moved three trees by hand. If I regenerate, do I lose that?**
Decided, and shown for points. A hand edit is kept as intent at a place (a pin or a clear), not
against a generated id (R3). In `example` it held through six rule changes. Paths and painted
fields are open.

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
Since shown: a piece turned (`strait` `move_b`, 10 degrees) and pieces inside pieces (`atlas`);
the limits above may be stale, and the current ones are in `examples/atlas/README.md`. Loading
pieces separately while the game runs is by chunk (R48).

**Does it merge cleanly in version control?**
Decided in shape, open in practice. Files are flat TOML keyed by id, so a change is a few lines in
one place (R1, principle 11), and layers split files by owner. Whether teams merge without pain is
for real users to say (studios later).

**How do we review a change to a map?**
Shown, partly. Named views are re-rendered and diagnostics compared. Nothing shows a map diff.

## My own code

**I need something you do not have. Am I stuck?**
Shown. You can define new kinds as data, add fields, attach your own data to anything, and join
the build with your own script. (`example`, `city`)

**In what language?**
Decided. Python, through a thin SDK (R2, R19); a plain command that reads and writes files also
joins the build, in any language (R22). The engine is Rust (R19); you never write Rust.

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
Shown. A generate step runs at its layer, reads what is at or below it, and returns rows and
features the layers above build on (R21, R22, R32; `spec/protocol.md`). (`city`, `massif`)

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
Shown: clumps and growth by cause, the default to teach (R5, `glade`). Open: undergrowth that
follows the trees and a reusable "biome" that bundles fields and scatters. Decided: a hand-moved
tree is kept as a pin when the scatter runs again (R3).

**Rivers and lakes?**
Shown. A lake can be given a shape, or left to fill the ground and find its own shore. A flowing
river, stated by its ends over simulated land, is shown partly (`basin`, `townland`).

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
Shown, partly. The player map is data per floor (rooms, walls, doors, stairs, labels) that a game
restyles, and a Godot minimap switches floor as the player climbs (R47, `station`). Revealing rooms
as visited is the game's; we give a discovery grid.

**Can it check that the player can actually get somewhere?**
Shown, partly. You state who walks and where they must reach. It reads the ground only.

**Does it support weather?**
Not ours, for weather that happens: rain, falling snow, wind, changing light are the engine's.
What we can give it: a world in another state, as a variant (a winter map with snow on the ground
and bare fields, `vale`, `example`); and fields a weather system can read, such as wetness or
where snow would lie. Sun and sky hints are the engine's side of the door, passed through outside
the world file (doctrine section 6).

## Using it with an AI agent

**Is it made for agents?**
Shown. Every world here was written by one, working from diagnostics and rendered views.

**Can an agent work from a concept image?**
Open, for the owner: it is in no milestone (challenge 10). What would be ours: a view matched to
the image's viewpoint, the render beside the image, and a measure of how far they differ.

**What stops an agent from making a mess?**
Shown, partly. Every failure names the place and what to change (principle 8); a step runs only
once trusted (`spec/protocol.md`); a change report says what moved; budgets and checks fail the
build. Fresh agents built from the documents at the M5 and M6 closes, and what they tripped on was
fixed (`TODO.md`). At the M7 close four agents of two model families (both Claude models) did the
same: all four worlds built, 31 trips for one family and 22 for the other, 8 met by both
(`docs/challenges.md` row 51).

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
Decided: an editor window, painting or nudging in the engine as the source (text is the only
source, principle 1), worlds generated while the game runs, and any look we do not place
(`docs/charter.md`).

**What does it cost? What is the licence?**
Decided for the code: MIT (`LICENSE`), at no cost. Open, for the owner: anything beyond the code.

**Who else uses it?**
Open, for real users: nobody outside the project yet. M7 runs the first outside agents.

**What happens to my worlds if you stop?**
Shown. You keep the documents (TOML), the builds (rasters, JSON, mesh buffers) and the code (MIT).
Nothing needs a service of ours.

## Asked by two outside readers

Two fresh readers were given one paragraph about the tool and a role, and nothing else: S, a senior
developer at a large studio (who assumed a proprietary engine; read those questions as Unreal); B, an independent developer with no tools
programmer. Their questions, where the sections above did not already answer them. A blank answer
means we have not faced the question.

| | Question | Mark | Answer |
|---|---|---|---|
| S | Is this meant to replace our world editor, sit in front of it, or run beside it? | Decided | In front of it: the text is the only source (principle 1); the editor opens what the loader made, and nothing reads its changes back (`docs/charter.md`). |
| B | Someone moves a rock by hand in the engine editor. What happens on the next rebuild? | Decided | It is lost (principle 1). Make the move in the text, as a pin or a new `at` (R3), or ask the agent to. |
| S | How does a level artist nudge a rock 30 cm in a text file? | Shown, partly | By asking the agent, or editing one number, or a pin on a scattered rock (R3, `example`). Open, for real users: whether an artist accepts it. |
| B | Can I lock an area as finished while the rest keeps regenerating? | Decided | Yes. A lock is a replayed record, and a rule change reports what it would have done there (R4). |
| B | I change one line. How do I see what changed before I accept it? | Shown | New diagnostics, a change report of what moved (`changes.json`, `spec/graph.md` 6.4), and every named view rendered before and after (`engines/godot/sheet.sh`). |
| S | I upgrade the tool mid-project. Does my shipped world move? | Decided, partly | Same text, same bytes for one engine version (R26); a release that may change any value raises the numerics epoch, so nothing old is reused unseen (`spec/graph.md` 3.3), and the change report says what moved. Open, for the owner: whether a release may move a world at all. |
| B | Does it work without an AI agent? | Shown | Yes. The inputs are plain text files and the compiler contains no model. The agent is a convenience for writing them. |
| S | Is there AI inside the build itself? | Shown | No. The build is a deterministic program. A model was only ever used to write documents, and once to restyle a map picture. |
| S | Where does the agent run, and what does it see? | Not ours | You choose and host the agent. The compiler runs locally and sends nothing anywhere. |
| S | Is anything in the output generated by a model? | Shown | No. The output is your assets arranged, plus terrain and meshes computed from your text. |
| B | Do I have to disclose AI on a store page? | Not ours | That is between you, your agent and the store. Nothing in the build is model-made. |
| S | Who is accountable for the content? | Not ours | Whoever reviews and ships it. |
| S | A ten-thousand-line diff tells nobody whether a town looks right. What does a reviewer look at? | Shown, partly | Named views re-rendered from the same spots, and the list of diagnostics. There is no visual diff. |
| S | What catches a road that reads fine in text and cannot be driven? | Shown, partly | Checks you can state: grade limits, clearance under a bridge, reach on foot. Nothing checks driving. |
| S | How long from changing a line to seeing it? Is the rebuild incremental? | Shown, partly | Incremental: only what a change reaches is made again, and the result equals a clean build (`spec/graph.md` 7). An unchanged `atlas` builds in 1 s; a change to it still takes more than half a cold build (58% for `move_market`) (`TODO.md`, M6 close). |
| S | Forty people on one world. How do merges work? | Open | For real users (studios, later). Layers split the files by owner and refer only downward (doctrine section 3); two people changing things that depend on each other is untested. |
| S | Does it run headless on a build farm? Can builds be cached and shared? | Shown, partly | It is a command with an exit status, so headless works. There is no shared cache. |
| S | Does it know my budgets: memory per cell, draw calls, certification limits? | Shown, partly | It counts instances and triangles per chunk against limits you set. Memory, draw calls and certification are the engine's. |
| S | Fit by whose rules? Can I add rules, and can a failure block CI? | Shown | Its own seam and placement checks plus yours, written as lint scripts. Any error gives a non-zero exit. |
| S | What does it do that Houdini and our in-house tools do not? | Open | For the owner and real users. A claim, not evidence: the whole world as reviewable text, checks that name the cause, and an agent loop. It is not a replacement for a generator; no side-by-side exists. |
| S | Does the pack understand our asset database, variants, LOD chains, material instances? | Not ours, mostly | A pack is one glTF file per id with its lower-detail copies, measured facts and a taxonomy (`spec/assets.md` 2, 3.3, 5). The database, variants and material instances stay yours; a script can write the pack from them. |
| S | Does the output carry collision, navmesh inputs, streaming cells, audio zones, gameplay markup? | Shown, partly | Chunk grouping, solidity, your own markup passed through, and any field you export. The rest is not produced. |
| S | Can it produce a hero location designed on purpose, or only filler between them? | Shown, partly | Exact placement and landmarks the terrain gives way to. Sightlines, cover and pacing are not concepts. |
| S | Show me a world built with it beside one built by an environment team. | Open | For real users. There is none; our worlds are stand-in shapes. |
| S | What has shipped with it? | Shown | Nothing. |
| B | Who makes it, how many people, will it exist next year, do I get the source? | Open | For the owner. The source is MIT (`LICENSE`). |
| B | If I drop it after a year, what do I keep? | Shown | The build folder is plain rasters, JSON and mesh buffers, and the documents are TOML. Nothing is locked in a private format. |
| S | What does it cost me to find out it does not work for us? | Shown, partly | On Godot: build one of `examples/` and load it with `engines/godot`. On Unreal nothing exists (set aside, doctrine section 7). |
| B | Is there a ready importer for my engine? If not, how hard is it to write? | Shown, for Godot | `engines/godot/addons/via_loader`. A glTF reader (`engines/gltf`) shows the build is not shaped round Godot; the build format is `spec/build.md`. Unreal is set aside (doctrine section 7). |
| B | How much work is it to bind a bought pack of a few hundred meshes? | Shown, partly | One short entry per mesh (an id, a file, a taxon); every size is measured (`spec/assets.md` 2, 3), and `via pack check` lints the pack alone (8.1). Open, for real users: whether a pack written from a folder is needed. |
| B | What machine do I need? Windows, Mac, Linux? | Decided | All three are targets from the first day (R20, R26). Only a Mac has been tried so far. |
| B | Is there a complete example I can open in ten minutes? | Shown, partly | There is an example village and a viewer, but nothing is packaged for download. |
| B | What are the generated meshes, and are they game-ready? | Shown, partly | Bridge decks, walls, floors, stairs and cave rock. Flat shaded, no texture coordinates: stand-ins. |
| B | Will everything look samey? | Open | For real users. It looks like the assets and rules you give it; placement by cause is the default taught (R5), and a measure sits beside the eye where a look can be measured (R15). Our own worlds are grey boxes. |
| B | Can my quests and triggers hang on to places and survive a rebuild? | Shown, partly | Anything you name keeps its name. Scattered things have no stable identity. |
| B | Which AI does it need, and what does a day of work cost? | Not ours | Any agent that can edit files and run a command. Its cost is yours. |
| B | How long to learn enough of the format to review what the agent wrote? | Open | For real users. `spec/vocabulary.md` is the one page of words. |
| B | Who owns the world I build, and can I ship it commercially? | Decided, for the code | The tool is MIT (`LICENSE`). That your documents, assets and builds are yours is ours, with no ruling behind it: open, for the owner, as is a statement on the shipped stand-in pack. |
| B | Where is the honest list of what it cannot do? | Shown | `docs/charter.md` (what we do not promise), `docs/challenges.md`, each example's README, and every Open in this file. |
| B | Where do I go when I am stuck on a Saturday night? | Open | For the owner. There is no support channel; the diagnostics and `docs/reference/diagnostics.md` are what exists. |
| B | How many hours from download to walking around in something of my own? | Open | For real users. README's "First build of your own world" is the path; the M7 close times it for agents. |
