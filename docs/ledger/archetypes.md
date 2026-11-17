# Archetypes

The structural kinds every term in the [ledger](README.md) is filed under. A term names a concept
(a road, a tarn, a terrace). An archetype says what kind of thing that concept is, in words free
of any domain: a road is a Shape with a Profile, joined into a Network.

They are the layer between the forms content takes and the concepts a world is written in:

| Layer | What it is | Where it lives |
|---|---|---|
| Data types | The forms content takes | This page |
| Archetypes | Families of operation over those forms, each with a contract | This page |
| Mechanisms | The engine's form of an archetype | `spec/`, the engine crates |
| Concepts | A named use of mechanisms: a road, a tarn, a terrace | `lib/` |
| Worlds | What an author states | `examples/` |

Each layer is written only in the words of the one below it.

## What makes an archetype

One is admitted when it has five things.

1. **A definition:** what it reads and what it writes, in the data types below.
2. **Laws:** statements that can fail. What may move when an input moves, who gives way, whether
   names survive, what happens at its edge.
3. **A source:** its established name in the literature or in practice, or "ours" said outright.
4. **Independence:** it cannot be written as a combination of the others. If it can, it is a
   concept, not an archetype.
5. **Two genres:** concepts from at least two kinds of world file under it.

The definitions and sources are the domain's. The laws are this project's own statement of what
must hold when a world is changed and built again: the talks and papers read for this ledger
describe pipelines that build once, and say little about it. Full citations are in
`docs/references.md`, "Sources for the archetypes".

## Data types

| Name | What it is |
|---|---|
| Field | A value at every location. A raster field holds a number per grid post (a height, a mask). A label field holds one class from a fixed set, and cannot be blended or averaged. A share field holds a share of every class, summing to one. A volume field holds a value at every position in space, not only on the ground. |
| Shape | A point, a path or a region, carrying attributes. |
| Graph | Shapes joined at nodes, with topology: which edge meets which, and what they enclose. |
| Asset | A thing made elsewhere, with facts stated about it: its size, its front, where it joins others. |
| Instance | An asset at a place: a position, a facing, a size. |
| Mesh | A generated surface or solid that is not a height field: a rock face, a deck, a room. |

Fields and shapes are the two ways to describe space: a value everywhere, or discrete things
with identity (the field view and the object view; Couclelis 1992). Most of the craft is in
moving between them.

## The archetypes at a glance

| Archetype | In one line | Reads | Writes |
|---|---|---|---|
| Field | A value stated at every location | nothing, or position | a field |
| Shape | A point, path or region stated outright | nothing | a shape |
| Asset | A thing brought in whole, with facts | a file outside the world | an asset |
| Derivation | Content computed from other content by a fixed rule | fields, shapes | a field or a shape |
| Simulation | A process run in steps, carrying state | fields, conditions | fields and their by-products |
| Profile | A shape of change across the distance from a shape | a shape, a field | the field, or a mesh |
| Network | Paths joined at nodes; may read and write the ground | shapes, the ground, a cost | a graph, changes to the ground |
| Area partition | An area divided into pieces that tile it | a region and what divides it | regions, or a label field |
| Placement | One thing at one stated place | an asset, what it is placed against | an instance |
| Linear placement | Things at stations along a path, and spans between | a path, a spacing | instances and spans |
| Scatter | Many points over an area by density and spacing | a region, fields | instances |
| Recipe | What one thing expands into | a class, its list | parts |
| Grammar | A shape rewritten by rules into smaller shapes | a shape, rules | shapes, then instances or a mesh |
| Representation switch | A part one form cannot hold is given another | a region, a field | a mesh, a cut |
| Meeting | The form where two things join | both sides | the form between |
| Overlay | A named difference laid over a base | a base, a difference | the same world in another condition |
| Guidance | Design intent about what a player sees and where they go | nothing | an aim |
| Measure | A reading of the finished world | the built world | a number or a verdict, for a person |
| Markup | A fact carried to the game on a shape | nothing | data, handed on unread |
| Delivery | Packing the built world for the engine | the built world | files an engine loads |

The first three state content. The next thirteen make content from content. The last four read
the world and change nothing in it.

## Contracts

### Field

A value stated at every location: on the ground (a height, a mask, a share of each material) or
through space (a layer of rock).

- **Laws:**
  1. It has a value everywhere in the world; there is no hole and no value that is not a number.
  2. A label field is never averaged or blended. A share field sums to one.
  3. Changing the value at one place changes that place only, until something reads it.
  4. Its meaning does not depend on its resolution: the same statement at finer posts describes
     the same ground.
- **Source:** the field view of geographic space (Couclelis 1992); height fields and layered
  materials in terrain modelling (Galin et al. 2019; Peytavie et al. 2009); noise as a function
  of position (Musgrave et al. 1989).
- **Is not:** a Derivation, which is computed from other content.
- **In two genres:** a painted mask for where a wood grows; a height limit per city district.

### Shape

A point, a path or a region stated outright, with attributes. A path carries them along its
length.

- **Laws:**
  1. A path has a start, an end and a length. Any spot on it can be named by distance along it
     and offset to a side.
  2. A region has an inside, and whether a place is inside is never in doubt at its edge.
  3. Moving one part of a shape moves only what was stated against that part.
  4. A shape keeps its name however its geometry is edited.
- **Source:** the object view of geographic space (Couclelis 1992); road alignment as a
  horizontal and a vertical curve (civil design practice; OpenDRIVE's reference line).
- **Is not:** a Network (shapes joined at nodes) or an Area partition (regions that tile).
- **In two genres:** a trail's centreline; the outline of a walled yard.

### Asset

A thing made elsewhere and brought in whole: a model, a kit piece, a texture. It is opaque: it
can be placed and measured, not reasoned into.

- **Laws:**
  1. A world names an asset by its id or by where it is filed, never by its file.
  2. Everything a rule needs to know about it is a stated or measured fact. A missing fact is
     reported, not guessed.
  3. Swapping one asset for another with the same facts changes the look and nothing else.
  4. A revised asset names every place it changes.
- **Source:** the contract of modular kits in game practice (`docs/references.md`, "City building
  and asset kits"). Ours as an archetype: no source read states these laws.
- **Is not:** a Placement, which is an asset at a place.
- **In two genres:** a boulder; a wall bay of a town kit.

### Derivation

Content computed from other content by a fixed rule, with nothing carried from one run to the
next. It has four classes, by how far a change can reach. The first three are map algebra's
(Tomlin 1990); later texts add the fourth.

| Class | A place's value depends on | Examples |
|---|---|---|
| local | the same place in other fields | a threshold, a class from height and slope, the strongest share |
| focal | a neighbourhood round it | slope, curvature, a smoothing, the commonest class nearby |
| zonal | everything in its zone | a total per basin, a mean per district |
| global | places anywhere | distance to the nearest road, water gathered from upstream, a hollow filled until it spills |

- **Laws:**
  1. Same inputs, same result. It holds no state of its own.
  2. A change to an input can change the result no further than its class allows: the same
     place, the neighbourhood, the zone, anywhere.
  3. It never changes what it reads.
  4. Where a derived value and a stated one disagree, the stated one stands.
- **Source:** map algebra (Tomlin 1990) for the local, focal and zonal classes, and the texts
  that followed it for the global one; geomorphometry for the terrain measures; flow routing
  (O'Callaghan and Mark 1984) and depression filling (Barnes et al. 2014) as global cases.
- **Is not:** a Simulation, which iterates and carries state; nor a Measure, which nothing in
  the world reads.
- **In two genres:** wetness, for where reeds grow; distance to the nearest street, for what a
  lot is worth.

### Simulation

A process run in steps over fields, carrying state from one step to the next. It also gives off
fields that explain what it did: where water ran, where material settled.

- **Laws:**
  1. Its result cannot be had without running it; no step can be skipped.
  2. A change to any input can change the whole result.
  3. What the author states as fact (a lake here at this level, a river from here to there)
     holds in the result. The process settles the rest round it.
  4. Same inputs and same seed, same result, however many threads run it.
- **Source:** erosion as physical simulation (Musgrave et al. 1989; Mei et al. 2007); the
  simulation family of terrain methods (Galin et al. 2019); stand dynamics for forests (Oliver
  and Larson). Law 3 is ours.
- **Is not:** a Derivation (no state), nor Runtime (it runs while the world is built, not while
  the game runs).
- **In two genres:** water cutting gullies into a hillside; a forest thinning itself over years.

### Profile

A shape of change stated across the distance from a path, a point or an outline: a cross-section
carried along a curve, a brush's falloff, the bank round a levelled pad.

- **Laws:**
  1. Its effect ends at a stated distance. Beyond it nothing changes.
  2. At its outer edge it meets what was there with no step, unless a step is stated.
  3. Moving the shape moves the change with it, and the ground it left is as it was.
  4. Where two profiles claim the same ground, which gives way is stated. A clash between two
     firm ones is an error.
- **Source:** the typical section and the corridor of road design; sweeping a section in solid
  modelling; feature curves with side profiles in terrain modelling (Hnaidi et al. 2010).
- **Is not:** Linear placement (things at stations), nor a Meeting (the form where two profiles
  join).
- **In two genres:** a street's carriageway, kerb and footway; a trail's worn tread.

### Network

Paths joined at nodes into a graph. It may read and write the ground it runs on: a river carves
and never climbs, a road is graded to a limit. It is directed where flow has a direction (water,
traffic lanes) and undirected otherwise.

- **Laws:**
  1. Every edge ends at a node. Two edges cross only at a node, or on different levels.
  2. In a directed network of water, every edge runs downhill to an outlet.
  3. Moving one node moves the edges that end there and nothing else. An edge found by cost is
     found again.
  4. A node, an edge and an enclosed area each keep a name that does not shift when an
     unrelated edge is added.
- **Source:** stream networks and their ordering (Strahler 1957); rivers laid out before the
  terrain (Génevaux et al. 2013); roads found by cost (Galin et al. 2010) and grown by rule
  (Parish and Müller 2001); lanes and junctions in a road description (OpenDRIVE).
- **Is not:** a Shape (one path), nor an Area partition (the faces a network may bound).
- **In two genres:** a river and its tributaries; a town's streets.

### Area partition

An area divided into pieces that do not overlap and leave nothing over, each with a class. The
division may come from the edges of a network (blocks), from subdivision (lots), from a rule
over fields (biomes) or from the author's hand (themed regions).

- **Laws:**
  1. Every place in the area is in exactly one piece.
  2. A piece keeps its name when a piece elsewhere is split or moved.
  3. A change to what divides the area changes only the pieces that touch the change.
  4. Where a stated class and a derived one disagree, the stated one stands.
- **Source:** subdivision of parcels (Vanegas et al. 2012); nearest-seed diagrams; the
  classification of climate into biomes (Whittaker 1975).
- **Is not:** an Overlay (whose pieces may overlap), nor Markup (a region that only carries
  data).
- **In two genres:** biome classes over a mountain; lots along a street.

### Placement

One thing put at one stated place, most often by its relation to another: on it, beside it, at
its end, facing it.

- **Laws:**
  1. It follows what it is placed against. When that moves, it moves with it.
  2. No rule moves or removes it. A rule that would is reported.
  3. Two placements that claim one spot are an error.
  4. It keeps the name it was given.
- **Source:** constraints between drawn parts (Sutherland 1963); set dressing by hand in level
  art. Ours as an archetype: the sources read describe rules and treat the one-off as an
  exception.
- **Is not:** a Scatter (many, by a rule), nor a Recipe (a group as one).
- **In two genres:** a specimen tree on a hilltop; a lamp beside a door.

### Linear placement

Things repeated at stations along a path, and spans between them: posts and rails, poles and
wire, road markings, a row of trees.

- **Laws:**
  1. A station is named by distance along the path and offset to a side. It faces along the
     path or across it.
  2. Lengthening or bending the path adds or moves stations near the change. The far end keeps
     its own.
  3. A run starts and ends with its end pieces, whatever the length between.
  4. Nothing is placed where the path crosses something that needs the room, such as a gate or
     a junction, unless stated.
- **Source:** linear referencing (ISO 19148); fence and spline-mesh tools in engines; span rules
  in bridge design.
- **Is not:** a Profile (a continuous section), nor a Scatter (over an area).
- **In two genres:** a field fence with its rails; lamps along a street.

### Scatter

Many points drawn over an area by a density and a spacing rule. Each becomes an instance with
its own size, facing and variant.

- **Laws:**
  1. Same seed, same points.
  2. A change to a field moves only the points near the change.
  3. No two points stand closer than the spacing, and none where the ground is kept clear.
  4. What was placed earlier is not moved by what is placed later: larger things claim room
     first.
  5. One point can be removed or pinned by hand, and the edit holds while its place holds.
- **Source:** spatial point processes with marks, the statistics name for points that each
  carry values (Illian et al. 2008); Poisson-disk sampling (Cook 1986; Bridson 2007); plant
  ecosystems by competition (Deussen et al. 1998); placement by density maps (van Muijden
  2017). Law 5 is ours.
- **Is not:** Linear placement, nor a Placement.
- **In two genres:** undergrowth beneath a canopy; clutter across a market square.

### Recipe

What one thing expands into: the ground and growth of a biome, the parts of a bridge, the
pieces of a set placed as one.

- **Laws:**
  1. The same class expands the same way wherever it stands.
  2. Changing the recipe changes every place that uses it, and nothing else.
  3. A part can be named from outside (the door of this house) and keeps that name.
  4. The whole moves as one.
- **Source:** prefabs and assemblies in engines and in design software; biome tables in
  procedural placement (Carrier 2018).
- **Is not:** a Grammar, which rewrites again and again by rule; nor a Placement.
- **In two genres:** a farmstead; what "alpine meadow" means in plants and ground.

### Grammar

A shape rewritten by rules into smaller labelled shapes, again and again, until what is left
can be placed.

- **Laws:**
  1. The result follows from the starting shape and the rules alone.
  2. A change to the starting shape re-derives everything below it. A change to one rule
     re-derives only what that rule made.
  3. Every piece can be traced to the rule that made it.
  4. A piece edited by hand stays edited when the rules run again, or the clash is reported.
- **Source:** shape grammars (Stiny and Gips 1972); L-systems (Lindenmayer 1968); split
  grammars and CGA shape (Wonka et al. 2003; Müller et al. 2006); model synthesis and wave
  function collapse (Merrell 2007; Gumin 2016). Law 4 is ours.
- **Is not:** a Recipe (one fixed expansion), nor an Area partition (one level of division).
- **In two genres:** a facade split into floors and bays; a branching plant.

### Representation switch

A part of the world that one form cannot hold is given another. A height field cannot overhang,
so a cliff, a cave or a bridge deck becomes a mesh.

- **Laws:**
  1. Where the two forms meet there is no gap and no doubled surface.
  2. Whatever asks "what is the ground here" gets one answer per level.
  3. The switch covers only the part that needs it.
  4. Things can stand on the new surface as on the old.
- **Source:** cliffs, overhangs and caves in terrain modelling (Peytavie et al. 2009; Galin et
  al. 2019); extracting a surface from a volume (Lorensen and Cline 1987; Ju et al. 2002).
- **Is not:** Delivery, whose simpler copies for distance change no content; nor a Meeting.
- **In two genres:** a sea cave; a viaduct over a street.

### Meeting

The place where two things join. Its form belongs to neither side: a junction, a shore, a
doorway, the foot of a wall.

- **Laws:**
  1. Each meeting has one stated owner. Neither side builds it twice.
  2. Which side gives way is stated. Where neither may, it is an error.
  3. When either side moves, the meeting is made again.
  4. A meeting nobody owns is reported: a road that enters water with no ford and no bridge.
- **Source:** junction and kerb geometry in road design; the seam as the recurring fault of
  assembled worlds ([section 19](seams.md)). Ours as an archetype.
- **Is not:** a Placement (which follows one side), nor a Profile.
- **In two genres:** where a trail meets a road; where a house meets a slope.

### Overlay

A named difference laid over a world without rewriting it: a season, a state after an event, a
layer with another owner, a hand edit.

- **Laws:**
  1. The base is unchanged. Removing the overlay restores it.
  2. Only what the overlay states differs. The rest follows the base, later changes to the base
     included.
  3. Where two overlays disagree, their order decides, and the order is stated.
  4. An overlay that names something the base no longer has is reported.
- **Source:** composition by layers and variants (Universal Scene Description); edit layers on
  terrain and data layers in engines.
- **Is not:** the Principle of non-destructive editing, which is the property an overlay is
  made with; nor Runtime state.
- **In two genres:** winter over a valley; a festival dressing a town square.

### Guidance

Design intent about where a player goes and what they see: a landmark, a reveal, a pace. It is
stated, not built.

- **Laws:**
  1. It changes no content by itself.
  2. It is judged by a person. A number may inform the judgement and never replaces it.
  3. Where it can be tested, it is written so that it can be: visible from here, reachable
     within this.
- **Source:** what makes a city legible (Lynch 1960); serial vision (Cullen 1961); prospect and
  refuge (Appleton 1975); mystery (Kaplan and Kaplan 1989); level design practice
  (`docs/references.md`).
- **Is not:** a Measure (the reading), nor Markup (data the game reads).
- **In two genres:** a tower seen from every valley; a street that bends to hide the square.

### Measure

A reading of the finished world that changes nothing: what is visible, what can be reached, how
far, how many.

- **Laws:**
  1. Nothing in the world depends on it.
  2. It names the place it speaks of, and what would change the answer.
  3. It states what it assumed: the body that walks, the height of the eye.
  4. It reports. Whether the world passes is a person's call, except where the author stated a
     limit.
- **Source:** isovists (Benedikt 1979); space syntax (Hillier and Hanson 1984); viewshed
  analysis; reachability checks in level design. Laws 2 to 4 are ours.
- **Is not:** a Derivation, whose result later stages read.
- **In two genres:** can a walker reach the summit; is the cathedral visible from the gate.

### Markup

A shape or a tag that changes nothing in the world and carries a fact to the game: where things
appear, what a region starts, what can be used.

- **Laws:**
  1. It is handed on unchanged, and its meaning is the game's.
  2. It stays attached to its place or its thing when the world is built again.
  3. It keeps its name, so a save or a quest can point at it.
- **Source:** volumes, triggers and tags in level editors; smart objects. Ours as an archetype.
- **Is not:** Guidance (read by the author), nor a Placement (something seen).
- **In two genres:** a spawn zone in a wood; a shop's service point.

### Delivery

Packing the built world for the engine: tiles, simpler copies for distance, batches, the
player's map as data. It changes no content.

- **Laws:**
  1. The world is the same before and after. Only its packing differs.
  2. Pieces packed apart meet without a crack.
  3. A change to one place rewrites only the pieces that hold it.
  4. Same world, same files.
- **Source:** models held at several levels of detail (Clark 1976); tiled and streamed worlds in
  engines. Laws 3 and 4 are ours.
- **Is not:** a Representation switch (which changes content), nor Runtime (what the engine
  then does).
- **In two genres:** terrain tiles; one merged stand-in for a far town.

## Outside the content model

Three marks file what the model deliberately leaves out. They are not archetypes: they have no
contract over the data types.

| Mark | What it files |
|---|---|
| Runtime | What the engine or the game does while the game runs: streaming, weather, crowds, physics, light. The world holds only what these need as input. |
| Practice | How people work: a role, a stage of production, a habit of review. Not content. |
| Principle | A property asked of the whole: the same input gives the same world, a hand decision survives, a name persists. Each is a law of every archetype above, said once. |

A row that fits neither an archetype nor a mark is filed `-`. Such rows are listed in the
README, and are the signal that an archetype is missing or wrongly drawn.

## What every contract answers

Five matters belong to no one archetype. Each contract's laws answer them in its own terms.

| Matter | What it is |
|---|---|
| Reach of a change | When an input moves, how far the result may move. A world is revised all through production and, in a live game, after release. |
| Who gives way | When two things claim the same ground, which one yields: a road over a river, a pad under a house, a landmark over scenery. |
| Names | A made thing keeping its name when the world is built again, so edits, saves and quests still point at it. |
| Edges | Where a thing meets something else. The form there is where a world most often looks wrong. |
| Scale | Real sizes, compressed sizes, and the player's body as the unit. |

## Sources of content

Where each thing in a world comes from. Who writes a statement, a person or an agent, does not
change what kind of thing it is; and the difference between a realistic and a stylised open
world is one of degree.

| Name | Concept |
|---|---|
| Specified | Stated outright by the author: this shelf here, this bridge there, this landmark on that hill. |
| Derived | Computed from other content: flow from height, biome from moisture, a guardrail from a drop. |
| Imported | Made outside and brought in whole: a model, a kit piece, a texture. Opaque. |
| Specification density | How many specified statements a given area carries. The main dial between genres. |
| Derivation depth | How many layers are computed from the specified ones. |
| Space graph | The layout as places and links with the cost of crossing each: shelves and connectors, rooms and doors, regions and passes. |
| Dressing | Derived form laid over a specified shape: rock faces on a shelf, a kit on a building's mass, plants on a top. |
| Style substitution | A style as a set of swaps over constant archetypes: which concept fills each one. |
| Fixed neighbour | Specified or imported content that derived surroundings must meet and never alter. |
| Checkable intent | A design aim written so that it can be tested: reachable within the body's limits, a landmark visible from the path, rewards no farther apart than a set distance. |

How two kinds of open world differ on those dials:

| | Realistic, large | Stylised, compact |
|---|---|---|
| Specification density | Low: sparse features over a wide area | High: dense terraforming and many custom places |
| Derivation depth | Deep: erosion, abiotic data, ecology, scatter | Shallow: a top-and-side rule, dressing, accents |
| Imported content | Large libraries of natural assets | Kits per region, setpiece models |
| Total work | Larger | Smaller: the world is far smaller |

The same archetypes serve both; the dials are set differently.

## How the list has changed

| Date | Change |
|---|---|
| 2026-11-13 | First list: fourteen archetypes, read from studio talks and one review paper. |
| 2026-11-13 | Meeting added; 15 rows moved to it. |
| 2026-11-18 | Tightened to the five criteria and widened to file the whole ledger: twenty archetypes and three marks. Added: Asset, Placement, Overlay, Measure, Markup. Renamed: Curve to Shape, Field derivation to Derivation (now in four classes), Terrain-coupled network to Network. Guidance split: what is stated stays Guidance, what is read from the world is Measure. The marks Runtime, Practice and Principle file what the model leaves out. |
