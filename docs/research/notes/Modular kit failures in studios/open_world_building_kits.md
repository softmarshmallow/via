# Open-world and action games (non-Bethesda): trial-and-error accounts of exterior building kits and settlement construction

Reading notes for the report writer, before the six questions:

- Provenance tags used on every finding: **[first-hand]** = a named developer speaking or writing, page opened; **[session description]** = only the conference abstract could be opened, the talk itself could not; **[third-party notes]** = someone else's summary of a developer talk; **[search summary, page not opened]** = the claim comes from a search-result digest and the page itself returned an error or was not fetched; **[inference]** = mine.
- All pages were read through a fetch tool that summarises. Short quotes below are as that tool relayed them; check the wording against the page before printing any of them. The Rise of the Ronin article is Japanese and was read in machine translation.
- Yield was low. Most of the developer material on this subject is in conference video (GDC Vault, YouTube, Vimeo) that could not be opened as text. About 80 searches and page reads produced two rich first-hand accounts (Embark's The Finals, Team Ninja's Rise of the Ronin), a handful of single-point first-hand accounts, and many dead ends. The dead ends are listed under each Gaps heading so nobody repeats them.
- The six items named in the brief as already recorded (Watch Dogs 2 recipes, Wildlands layers, Far Cry 5 nature, Cyberpunk rejecting procedural tools, Witcher 3 off-grid placement, Spider-Man keeping hand work) are not repeated. No new failure story was found for any of them.

## Assassin's Creed series: how were Paris, London, Egypt, Greece, England, Baghdad and Japan built from kits, and what failed or was redone?

### Takeaway
The only opened first-hand accounts say the cities were hand-assembled by level designers and level artists from a modeller-made library of whole houses plus modular parts, under strict parkour collision metrics, with procedural tools kept to landscape, roads and vegetation. No developer account of a building generator, a roof kit redo, a classical temple kit or a Japanese castle kit could be opened; those questions remain unanswered.

### Cited Findings

Origins (Egypt)
- [first-hand] Oleg Gamov, senior level artist at Ubisoft Montreal, describes the building kits as a library of houses plus modular parts (balconies, roofs, windows, facades) made by modellers, and says there is no secret technology behind them. — [80.lv, Creating the World of Assassin's Creed Origins](https://80.lv/articles/creating-the-world-of-assassins-creed-origins/)
- [first-hand] Order of work: the level designer places the houses first, for gameplay; the level artist then dresses interiors and exteriors. Structure is decided before look. — [80.lv](https://80.lv/articles/creating-the-world-of-assassins-creed-origins/)
- [first-hand] Difficulty: parkour makes house creation hard from modelling through to in-game use. Cause: every surface is climbable. What they do: collision has to meet what Gamov calls "very strict and precise metrics". — [80.lv](https://80.lv/articles/creating-the-world-of-assassins-creed-origins/)
- [first-hand] Difficulty: procedural vegetation growing through houses. What they did: inhabited areas were excluded from the spline-defined vegetation zones and planted by hand, accepting that hand-placed assets cost more. — [80.lv](https://80.lv/articles/creating-the-world-of-assassins-creed-origins/)
- [first-hand] Roads are splines adjusted by hand in all three axes and projected onto the terrain; the base terrain came from procedural and terrain teams and was then reshaped by the level artist with gameplay first. — [80.lv](https://80.lv/articles/creating-the-world-of-assassins-creed-origins/)
- [first-hand] Raphael Lacoste, art director: every discovered location is handcrafted; a village or city is made by a level designer and a level artist. Procedural tools set "a basic recipe" for vegetation and rocks in open land. Because the world is seen from every side, "we have to build everything". — [GamesBeat, page 2](https://gamesbeat.com/how-ubisoft-created-the-art-for-the-massive-world-of-assassins-creed-origins/2)
- [first-hand] Lacoste: Origins was the first Ubisoft game to depict a whole country, so the team built a new procedural pipeline in the engine for forests, desert, mountains and roads; artists write the recipes and adjust output by hand; the team does not "want the tools to create everything". — [80.lv, The Way of Ubisoft: AI Tech or Artists?](https://80.lv/articles/the-way-of-ubisoft-ai-tech-or-artists/)

Unity (Paris), the monumental case
- [first-hand] Caroline Miousse, senior level artist, built Notre-Dame from blueprints, books and photographs assembled "like a puzzle". The interior was changed to add several layers of navigation; cables and incense were added across the second level so the player can move above ground; the spire was included although it did not exist in the period, because players should be able to climb it. The spire was the technically difficult part: it had to satisfy several traversal rules and smooth navigation. Her stated lesson: a monument has to work as a play space ("It has to be enjoyable"), and it must be recognisable while deliberately altered. — [SIGGRAPH blog, How Ubisoft Re-created Notre Dame](https://blog.siggraph.org/?p=5940)
- [search summary, page not opened] A Persian-language article is summarised as saying the Unity team kept medieval roof shapes but found them hard to traverse, and responded by reducing pointed structures and flattening sloped sections. Third-party, unverified. — [zoomg.ir](https://zoomg.ir/2014/12/12/809/assassins-creed-unity-paris/)
- [search summary, contradicted on opening] A search digest attributed to Ubisoft's own material the claim that most Unity buildings are scaled down by half or a third with Notre-Dame near one to one. The Ubisoft page it pointed to was opened and contains no such statement; it is about an architecture research project and a parkour maquette. Treat the scale claim as unsourced. — [Ubisoft News](https://news.ubisoft.com/en-us/article/6GWu5p7Yf1kzoAEcxYY3gd/how-assassins-creed-unity-can-break-down-architectural-boundaries)
- [search summary, page not opened: HTTP 403] Johanne Drolet's portfolio says she worked on Unity's architecture team as both level artist and modeller, building medieval and Renaissance house kits, roofs, fire walls and chimneys. This is the only evidence found that Unity's roofs, fire walls and chimneys were separate kit families. — [ArtStation](https://johannedrolet.artstation.com/projects/PeOK2Z)

Odyssey (Greece), the classical case
- [search summary, page not opened: HTTP 403] Olivier Carignan's portfolio describes a "Rich Villas" architectural kit of hundreds of wall, roof and column pieces that level artists used to populate cities and villages. No account of what went wrong with it. — [ArtStation](https://oliviercarignan.artstation.com/projects/PmmNDL)

Mirage (Baghdad)
- [search summary, pages not opened] Creative director Stéphane Boudon is reported saying dense structures, flat roofs and connections between buildings make Baghdad ideal for parkour; almost nothing from ninth-century Baghdad survives, so the city was built from research rather than survey. — [Den of Geek](https://www.denofgeek.com/games/assassins-creed-mirage-lost-city); [Apollo Magazine](https://apollo-magazine.com/?p=999371)

Shadows (Japan)
- [first-hand] Simon Lemay-Comtois, associate game director: in Shadows a surface with no physical handhold cannot be climbed, which he calls "a pretty big deal for us". The stated reason is design control, not architecture: it let the team build deliberate "parkour highways" and decide where each of the two characters can go. Most of the map stays climbable, but players sometimes have to look for a valid entry point. The grappling hook attaches to rooftop ledges. Castles are described as concentric stone walls enclosing climbable buildings (warehouses, barracks, watchtowers) and a keep. — [Ubisoft, Parkour System Overview, 7 January 2025](https://www.ubisoft.com/en-us/game/assassins-creed/news/4TA6gKaTvtOC1mOjZIxCZd)
- [first-hand] Pierre Fortin, technical architect: the engine no longer needs a low, medium and high version of each building; one system covers the range from two kilometres to ten metres. The world "is fully modeled, created by a team of great level artists", and unlike a corridor game it has to be built from all angles. Nothing in the interview on kits, roofs or castles. — [GamesBeat](https://gamesbeat.com/how-ubisoft-built-the-world-of-assassins-creed-shadows-pierre-fortin-interview/)

### Inferences
- [inference] Across Origins and Unity the binding contract on a house kit is traversal (collision metrics, handholds, roof slope), not appearance. A kit piece is accepted or rejected by what the player's body can do on it.
- [inference] Shadows' handhold rule moves climbability from "every surface" to "declared ledges". For a kit that is a change of contract: a piece must now state where it can be climbed. The article presents this as a design choice; whether curved Japanese roofs forced it is not stated and should not be claimed.
- [inference] The portfolio evidence (house kits, roofs, fire walls, chimneys as separate families on Unity; walls, roofs, columns on Odyssey) suggests roofs were a kit of their own on both games, but no developer explains why or what failed first.

### Gaps
- No developer account found for Syndicate (London) or Valhalla (England) building construction at all.
- No account found of an Anvil building generator, of a roof kit redone for parkour, of classical temple kits (column orders, pediments), or of Japanese roof and castle kits. Searches on these returned only engine and rendering material ([Inside Anvil, Ubisoft](https://news.ubisoft.com/en-us/article/6u0kflsdY20YlnZks7rUeG/inside-anvil-the-technology-powering-assassins-creed-shadows) was surfaced but not opened; the digest says it concerns rendering and model LOD).
- Could not open: The Verge's 2014 feature on Unity's Paris (fetch refused); both ArtStation portfolios above (403); the YouTube interview [Building Notre Dame (and Paris)](https://www.youtube.com/watch?v=KLKLOtxFGps) (video).
- How late changes to a kit piece spread through an Assassin's Creed city, and how hand dressing survives a kit change, is not addressed in anything opened.

## Ghost of Tsushima, Sekiro, Rise of the Ronin, Black Myth: Wukong or similar: what is said about building traditional East Asian architecture modularly? What did not fit a grid?

### Takeaway
Two first-hand accounts were found, and both report the same thing from different ends: generic rules did not produce Japanese places. Team Ninja's automatic street layout came out straight and unnatural and had to be driven from period maps; Embark's general roof node could not make layered Japanese roofs and got a dedicated node instead. Nothing was found on bracket sets, post-and-bay structure, or walled compounds with gates.

### Cited Findings

Rise of the Ronin (Team Ninja), Yokohama, Edo and Kyoto in the 1860s
- [first-hand, machine-translated] Hiroyuki Kato, environment art lead. The team split in two (open world, mission stages) and used Houdini to generate stages automatically as far as possible. Yokohama was built first and used as the model for the others; stages were revised many times. The world is cut into 2,048 m square tiles. — [CGWORLD vol. 313, part 2](https://cgworld.jp/article/202410-cgw313-rotr02.html)
- [first-hand, machine-translated] Difficulty: purely automatic placement produced straight, unnatural streets. What they did: drew a road texture from historical maps of Edo, generated block data from it in Houdini, and placed building assets on the blocks. — [CGWORLD](https://cgworld.jp/article/202410-cgw313-rotr02.html)
- [first-hand, machine-translated] Building placement on blocks is random under rules; the rule named is that identical buildings may not stand next to each other. After feedback from the development and art teams, houses were shifted and their colours and shapes distorted to remove the generated look. Street props were placed automatically from several prepared patterns. — [CGWORLD](https://cgworld.jp/article/202410-cgw313-rotr02.html)
- [first-hand, machine-translated] Rule for what may exist: anything that could not have been built with period technology is excluded; landmarks that did not exist at the time (the Edo Castle keep, a memorial hall in Yokohama) were added on purpose for authority and navigation. — [CGWORLD](https://cgworld.jp/article/202410-cgw313-rotr02.html)
- [first-hand, machine-translated] Terrain: height maps and erosion in Houdini; cliff masks above a slope angle; slope and ground-type attributes exported to decide where trees may grow. The article does not say how buildings meet sloped ground. — [CGWORLD](https://cgworld.jp/article/202410-cgw313-rotr02.html)

The Finals (Embark), Kyoto map
- [first-hand] Adrian Björkerud, technical artist. The general roof node builds a roof from four artist-supplied meshes: plane, ridge, fascia, eaves. Layered traditional Japanese roofs did not fit it. Rather than load the general node with those constraints, the team added a separate Kyoto roof node. — [SideFX, Making the Procedural Buildings of THE FINALS, 11 February 2026](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)

Assassin's Creed Shadows
- See the first question: handhold rule, castles as concentric walls with climbable outbuildings. — [Ubisoft](https://www.ubisoft.com/en-us/game/assassins-creed/news/4TA6gKaTvtOC1mOjZIxCZd)

Ghost of Tsushima and Ghost of Yōtei (Sucker Punch)
- [search summary, pages not opened] Artist portfolios mention "core architecture sets" and a "burnt architecture set" on Tsushima, which implies kit sets with a damaged variant, but say nothing on how pieces join. — [ArtStation](https://ragecageart.artstation.com/projects/OoGL5b)
- [search summary, pages not opened] For Yōtei, art director Joanna Wang is reported saying that familiar elements such as buildings had to be reconsidered for Hokkaido's open ground, including how buildings sit in open terrain. No structural detail. — [Postmode](https://postmode.org/ghost-of-yotei-interview-art-director-ainu-culture/); [Press Start](https://press-start.com.au/features/2025/10/02/ghost-of-yotei-interview/)

Black Myth: Wukong (Game Science)
- [search summary, pages not opened] Reported as built largely from scans of real temples and relics across several provinces (Dule Temple is named), with access to protected sites a difficulty. This is scanning whole structures, not kit assembly; no developer account of modular Chinese architecture was found. — [Unreal Engine interview](https://www.unrealengine.com/en-US/developer-interviews/black-myth-wukong-wows-with-ue5-early-access-visuals)

### Inferences
- [inference] Embark's decision is a clean example of a contract boundary: the four-mesh pitched roof contract (plane, ridge, fascia, eaves) was kept small and a second roof contract was added for the layered case. They judged widening the first contract to be worse than owning two.
- [inference] Team Ninja's fix puts the street network outside the generator: it is authored data (a map texture) that the generator reads. Their generated look was then removed by per-house offsets and distortion, which is variation applied after placement rather than inside the kit.
- [inference] The Ronin account never mentions a module or grid for buildings; placement is of whole building assets on blocks. The kit boundary there appears to be the building, not the wall panel.

### Gaps
- Nothing found from developers on curved roofs and eaves as kit pieces (corner eave pieces, hip and gable combinations), bracket sets, post-and-bay modules, verandas, or compounds with walls and gates. Nothing on what failed to fit a grid, stated as such.
- Sekiro: no developer account found. Ghost of Tsushima: the lead environment artist interview at DualShockers failed to load (connection dropped); Sucker Punch's GDC 2021 talks are video.
- Two Japanese-language searches for CEDEC sessions on Japanese roofs, castles or procedural towns found none for a shipped game. They surfaced only academic work (a procedural method for irimoya tile roofs; an interactive Japanese castle modeller), which is not developer testimony.
- A search for GDC material on procedural ancient Chinese architecture (NetEase, Tencent, Everstone) found only academic papers.
- CGWORLD's part 1 on Rise of the Ronin was opened and contains nothing on environment construction.

## Kingdom Come: Deliverance 1 and 2, The Witcher 3, Hogwarts Legacy, Elden Ring, Dragon's Dogma 2, Baldur's Gate 3 or similar: medieval towns and villages

### Takeaway
One real failure story was found: at Warhorse, enlarging the houses after the first quests were built broke those quests. Beyond that, Warhorse's accounts are about where the layout comes from (old military maps, woodcuts, surviving buildings), not about kit structure. No developer account was found for timber framing, pitched roofs over irregular plans, or houses on slopes.

### Cited Findings

Kingdom Come: Deliverance
- [first-hand] Tobias Stolz-Zwilling, PR manager at Warhorse, May 2017: once the first three quests are done "you find out that you need to make your houses a bit larger, but it completely destroys quest one"; with larger houses the non-player characters in that quest became confused. He frames open-world development as two steps forward and one back, because there are no closed levels to finish, test and lock one at a time. Martin Ziegler, technical designer, adds in the same interview that the hardest interactions between systems surface late. The page is a mirror and does not name the original publisher. — [interview mirror](https://fextra.pages.dev/posts/kingdom-come-deliverance-interview-with-warhorse-development-update-dlc-taking-chances-building-a-community)
- [search summary, page not opened] In Warhorse's Weekly Torch forum series a programmer is reported saying one low-detail system serves both vegetation and villages or towns. — [Warhorse forum](https://forum.kingdomcomerpg.com/t/warhorse-studios-weekly-torch/29593/144)

Kingdom Come: Deliverance II
- [first-hand] Ondřej Bittner, senior game designer: hills, valleys and rivers come from geographic data because they have changed little; village layouts come from mid-nineteenth-century military maps, which show individual buildings, roads and logistical points and usually record "fairly static villages". For surviving castles and churches the team asks the caretakers for documentation ("Sometimes they do, sometimes they don't!"); where there is none, the building is reconstructed from general knowledge of the period. The lost Kuttenberg clock tower was rebuilt from sixteenth-century woodcuts that show the town from a hill with little detail. — [Xbox Wire, 4 February 2025](https://news.xbox.com/en-us/2025/02/04/building-an-authentic-bohemia-in-kingdom-come-deliverance-2/)

Horizon Forbidden West (tribal settlements, the nearest fantasy-village material found)
- [third-party notes] An anonymous blog summarising Roland IJzermans's GDC 2022 art direction talk records that each tribe's architecture follows from how the tribe lives (woven huts that look grown for one tribe; angular structures around a central tower for another) and that concept artists worked closely with designers so settlements serve play. The summary has nothing on kits. — [The Wizards Code](https://thewizardscode.blogspot.com/2023/04/designing-settlements-in-world-of.html)
- [search summary, page not opened] A Guerrilla environment artist's portfolio describes assembling and kitbashing the Tenakth "decoration building blocks" into assemblies inside one settlement. — [ArtStation](https://niekschlosser.artstation.com/projects/8wob9E)

### Inferences
- [inference] The Warhorse story is a late contract change in its purest form: the footprint and interior size of a house was an unstated dependency of scripted behaviour (navigation, positions), so a geometry change invalidated content that never referenced the kit by name.
- [inference] Kingdom Come's settlement layout is surveyed, not generated: building positions come from a historical map. The generator-style question "where does a house go" is answered by data, as in Rise of the Ronin's Edo.

### Gaps
- No developer account found for Hogwarts Legacy hamlets (the Unreal Engine interview returned 403; a suspected SideFX talk does not appear to exist), Elden Ring, Dragon's Dogma 2 (a Japanese-language search for a CEDEC session found none) or Baldur's Gate 3.
- No account found of timber-frame kits, pitched roofs over irregular plans, or stone bases on slopes, for any of these games.
- The Witcher 3: nothing new beyond what the earlier pass recorded.
- Whether Warhorse builds houses from modular parts or as whole models is not stated in anything opened.

## Cyberpunk 2077, Watch Dogs 2 and Legion, Marvel's Spider-Man 1 and 2, GTA V, The Division 1 and 2, Mafia: modern city buildings

### Takeaway
The richest account found is not from an open-world game: Embark's building system for The Finals, five years in development, documents its piece contract, how hand edits survive regeneration, and several things that were tried and removed. For the named open-world cities, only Techland (Dying Light 2) and Frogwares (The Sinking City) gave opened first-hand accounts, and both describe a top-down plan-then-fill tool rather than failures inside the kit.

### Cited Findings

The Finals and ARC Raiders (Embark), first-hand, Adrian Björkerud, technical artist — [SideFX, 11 February 2026](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)
- Structure. One "Building Creator" node per building holds shared parameters (wall thickness, floor height) and makes no geometry. Inside it, one "feature node" per architectural element: exterior walls, floors, roofs, rooms, room styles, windows, doors. Each feature node has two inputs and two outputs: the building geometry, and a blockout or auxiliary stream used for alignment.
- Every building starts from a simple blockout mesh that fixes footprint, proportions and silhouette; all feature nodes read it as shared spatial context.
- Why not one tool: it did not start as a single all-in-one asset and grew into loosely coupled nodes. Stated benefit: a bug or an improvement can be isolated to one node, and a node can be replaced without breaking the rest. Most nodes are order-independent; the exception is where logic needs sequence (walls before windows).
- Facade lines. The walls node makes horizontal edge loops that separate storeys and a variable set of vertical loops that subdivide the facade; artists adjust vertical spacing per wall segment and per corner in the viewport.
- Floors. Slabs align to the inner faces of exterior walls; a clearance threshold skips a slab where there is not enough headroom.
- Openings. A window or door module is three things: the visible mesh, a boolean mesh that cuts the hole, and sockets where the engine spawns actors. Because the hole is cut by a mesh, an opening can have any profile (arch, round).
- Roofs. Four artist meshes (plane, ridge, fascia, eaves). Layered Japanese roofs got their own node (see the East Asian question).
- Hand edits. Viewport edits are stored as simple geometry in parameters on the tool, in world space, not as references to indices of the input geometry. Result: an edit stays put when the underlying building is regenerated, as long as the feature it belongs to is still in the same place. The article does not say what happens when the feature moves far.
- Late change. Feature nodes declare dependencies, so a small change recooks only the affected part; a change to a global parameter reaches every feature node. Blockout to finished fractured asset takes about four to six minutes per change.
- Tried and removed: furniture. Players at full sprint kept snagging on chairs and tables, so furniture was stripped.
- Tried and changed: free interiors on the Monaco map. With more than thirty enterable buildings, varied interiors confused players. Fix: one interior rule for all Monaco buildings (entrance opens to a hallway, stairs on the left, exit on the right, attic hatch at the top of the stairs).
- Rejected: hand-made collision. Replaced by automatic decomposition into convex pieces per feature.
- Constraint from destruction: facades are not enough; buildings need full interiors. Input must be watertight so inner faces can be generated. Mouldings and door frames must be part of the main wall mesh so they break with the wall.
- Reuse in a second game. For ARC Raiders, which has no full destruction, the same tools output only the base building mesh and details became instanced meshes.
- Stated lessons: put a usable tool in artists' hands early ("The fastest way to make an unused tool is to over-polish it before showing it"); keep nodes loosely coupled; name nodes and meshes after real building components so intent is clear.

Dying Light 2 (Techland)
- [first-hand] Katarzyna Tarnacka, environment art director: designing buildings for an open world, first-person view and parkour together is hard; the standing problem is buildings that look like buildings and are also climbable. Level design needs shapes that "usually don't look like a city". Before production the team moved paper building shapes around to balance departments; in production a grey-box gameplay prototype is reshaped by concept artists and level designers until it reads as a city. Rooftops are treated as the main level. — [Techland blog](https://techland.net/blog/the-development-process-behind-dying-light-2s-environments-and-buildings)
- [first-hand] Łukasz Burdka, senior technology programmer: the CityBuilder tool takes a top-down approach in which users draw a city plan as an urban planner would, and it automates the repeatable part so designers do not place every wall. Changes to the visual style or geometry of buildings can be propagated across the whole city. — [Wccftech Q&A](https://wccftech.com/the-big-dying-light-2-qa-techland-opens-up-on-c-engine-tech-delays-game-length-much-more/)
- [search summary, page not opened] Tarnacka-Polito is also reported saying procedural generation was useful for finding the base geometry, the gameplay rhythm and the general shape, with hand work after. — [80.lv](https://80.lv/articles/dying-light-2-art-direction-and-technical-side-of-the-game)

The Sinking City (Frogwares), a 1920s city
- [first-hand, partial] Aleksey Yurkin (lead of level modellers) and Max Rybalchenko (senior tool programmer): step one is a library of building elements; step two is prefabs, which are presets of the buildings found in one kind of neighbourhood (rich houses, poor shacks, destroyed or intact, commercial or housing, a given style); step three is a two-dimensional vector plan of the city that marks which area type goes where. The excerpt opened stops there; Epic's full article returned 403. — [80.lv excerpt](https://80.lv/articles/ue4-tool-behind-the-streets-of-the-sinking-city); [Epic original, not opened](https://www.unrealengine.com/en-US/developer-interviews/discover-how-frogwares-city-generator-is-saving-valuable-time-during-development-of-the-sinking-city)

Marvel's Spider-Man 2 (Insomniac)
- [session description] David Santiago's GDC 2024 talk says the game used more than a dozen procedural systems for the open world and that the talk draws on "successful and less than successful examples". The content is behind the GDC Vault login and was not read. — [GDC Vault](https://gdcvault.com/play/1034803/Technical-Artist-Summit-Procedural-Tool)
- [search summary, page returned 404] A newspaper piece is summarised as saying that adding Brooklyn and Queens meant learning the geometry of brownstones and row houses, and that Houdini took over tasks such as placing street lamps. — [San Juan Daily Star](https://www.sanjuandailystar.com/post/spider-man-2-s-new-york-is-a-web-of-skyscrapers-and-brownstones)

Watch Dogs: Legion (Ubisoft)
- [first-hand, general] A Ubisoft Singapore article says procedural generation is indispensable at this scale but people set its variables (road width, traffic, wall soiling, distribution) and artists place or move key buildings for composition; the tool is described as an exoskeleton for artists. No kit detail. — [Ubisoft Singapore](https://ubisoftsingapore.com/post/how-realistic-in-game-cities-are-made)
- [first-hand] Ubisoft's article on "the tools that built London" is about character generation only; the one world remark is that getting tagging information into the world was a tremendous feat. — [Ubisoft News](https://news.ubisoft.com/en-ca/article/4po3S9Pwp1YcgBmGPmQxAh/watch-dogs-legion-the-tools-that-built-london)

Older and adjacent
- [session description] James Golding, Epic Games, GDC 2010, "Building Blocks: Artist Driven Procedural Buildings": a designer places simple shapes for each building; a technical artist writes rules that cut rectangular regions from the faces and split them recursively; each final region is filled from a library of feature meshes. The abstract names the hard cases as trim, roofs and non-rectangular faces. The lowest level of detail is the designer's original shape. The game is not named on the page. — [GDC Vault](https://gdcvault.com/play/1012219/Building-Blocks-Artist-Driven-Procedural)
- [first-hand documentation] Epic's City Sample guide (the Matrix Awakens city): zones set a maximum building height with a falloff curve; a lot processor sets lot size, setbacks from the pavement and freeway, building style, height and size; changing these regenerates the building volumes and lots, which can take several minutes for a large city. The page has nothing on the module kit itself. — [Epic documentation](https://dev.epicgames.com/documentation/unreal-engine/city-sample-quick-start-for-generating-a-city-and-freeway-using-houdini)

### Inferences
- [inference] Embark's edit storage is the most transferable detail found: an edit is anchored to a place in the world and carried as data on the tool, not to an element number in generated geometry, so regeneration does not orphan it. Its stated limit (the feature must stay roughly where it was) is the honest boundary of that approach.
- [inference] The Monaco fix is a contract added for players, not for artists: the ground floor's arrangement became a rule because variation between neighbours, usually a goal, was the fault.
- [inference] Three independent accounts (Embark's blockout, Epic's designer shapes, Techland's drawn plan) start from a coarse authored volume or plan and treat the kit as decoration of it. None starts from the pieces.
- [inference] Epic's 2010 abstract and Embark's 2026 article name the same hard cases sixteen years apart: roofs and faces that are not rectangles.

### Gaps
- No developer account found for GTA V, The Division 1 or 2, or Mafia building construction. Hangar 13 results were about destruction simulation and a fence tool; a search on Mafia: The Old Country found only polygon counts and setting.
- Facade composition as floors and bays, ground floors that differ, and corner buildings: only Embark touches these (per-corner loop spacing, a fixed ground-floor rule). No account found of corner-building failures in any open-world city.
- Why procedural tools were adopted or rejected: nothing new beyond the earlier pass.
- Could not open: Insomniac's 2019 and 2024 talks (video or login; a transcript service returned an empty page); Epic's Matrix Awakens talk at SideFX (page has only an abstract); Epic's Frogwares article (403).
- Out of scope but surfaced: an 80.lv interview with Joel Burgess (then at Ubisoft Toronto) states that changing a pivot, a footprint size or transition metrics late can break hundreds or thousands of built layouts, and warns against "patch pieces". His experience is Bethesda's, which this note excludes; it is mentioned only so the writer knows where it is. — [80.lv](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)

## Horizon, Death Stranding, Half-Life: Alyx, The Last of Us Part II, Satisfactory, Alien: Isolation or similar: industrial structures, pipes, gantries, ruins

### Takeaway
Almost nothing first-hand was found on kits that are not buildings. The usable material is Embark's account of what destruction demands of joined pieces, a thin Creative Assembly statement on Alien: Isolation's kit pipeline, and unopened patch-note evidence that Satisfactory had to add a world-wide grid for foundations after shipping without one.

### Cited Findings
- [first-hand] Embark, on pieces that must hold together in three dimensions under destruction: geometry is fractured ahead of time, not at run time, because run-time fracture is too costly and unpredictable for fast multiplayer; seams stay hidden until damage; collision is built per feature from planar data, split into convex pieces and extruded, so pieces stay valid after fracture, with a later pass merging neighbours. A detail that is not part of the main mesh does not follow the broken wall. — [SideFX](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)
- [first-hand] Mike Bailey, technical director at Creative Assembly: the first Alien: Isolation built its assets and modular kits with what was then a new pipeline (weighted normals on bevelled low-density meshes, tiling materials, floating decals), which is what made the kits highly reusable and consistent. For the sequel, art director Ana Sopikova says the team is working out how "classic modular environments could look in harsh planetary conditions", that is, the interior kit taken outdoors. No detail on how pieces join. — [80.lv](https://80.lv/articles/how-alien-isolation-2-expands-on-the-original-s-terrifying-atmosphere)
- [search summary, pages not opened or failed] For the first Alien: Isolation, the film set was reportedly broken down into "meaningful chunks" as the basis of the kit, and the team is reported to have used the film-industry habit of kit bashing, restricted to objects from before 1979. — [PCGamesN, returned 404](https://www.pcgamesn.com/alien-isolation/sevastopol-station); [Gamerboom summary of a Develop event](https://gamerboom.com/archives/82758)
- [search summary, pages not opened] Satisfactory: a generic world grid for foundations, used by holding a key while placing, arrived in an experimental build in November 2021, after years of early access; a later note restricts foundations to quarter-turn rotation when aligned to the world grid. Players report that one-metre foundations did not line up with two- and four-metre ones on the world grid until a later fix, and that there is no vertical world grid. The patch notes are Coffee Stain's; the reasons and the one-metre claim are players'. — [patch notes mirror](https://redlib.cypherpunktools.com/r/SatisfactoryGame/comments/qr0msq/patch_notes_early_access_experimental_v0508_build); [SteamDB patch notes](https://steamdb.info/patchnotes/7731109)
- [search summary, page not opened] Half-Life: Alyx: Valve's workshop release notes are reported as showing parts of the level art pipeline, namely tile meshes, static and dynamic cables, and texture hotspotting. — [Valve developer wiki](https://developer.valvesoftware.com/wiki/Half-Life:_Alyx_Workshop_Tools/Examples/en)
- [search summary, page not opened] Horizon Forbidden West: Burning Shores: an artist describes designing a kit of about ten unique props that make up most of one bespoke machine interior. — [ArtStation](https://olmo.artstation.com/projects/n06Wke)

### Inferences
- [inference] Satisfactory is a natural experiment in local versus global alignment: pieces that snapped only to each other could not be made to meet when two builds started in different places, and the remedy was a single world grid added later. The quarter-turn restriction and the one-metre mismatch report suggest that a world grid then constrains rotation and forces every piece size to be commensurate. This rests on unopened patch notes and player reports.
- [inference] Embark's destruction rule generalises to any kit whose pieces must behave as one body: attachment has to be a property of the mesh, not of placement.

### Gaps
- No developer account found for Horizon Zero Dawn or Forbidden West industrial ruins and cauldrons, Death Stranding structures, or The Last of Us Part II. Guerrilla's GDC 2024 slides on relic ruins exist as a PDF but exceed the fetch size limit and were not read. — [Guerrilla PDF, not opened](https://www.guerrilla-games.com/media/Team/Daniel%20Wewerinke%20-%20GDC24%20-%20Relic%20Ruins%20-%20Creating%20Environmental%20Puzzles.pdf)
- No developer account found for pipes, gantries, conveyors or other pieces joined freely in three dimensions. A search surfaced only a hobbyist's lessons on modular pipes.
- Half-Life: Alyx: no developer talk or interview on its kits was found.
- A search for S.T.A.L.K.E.R. 2, Atomic Heart and Metro Exodus environment kits found nothing on kits.

## Across all of them: late changes to a kit piece's contract, streets or terrain moving under finished buildings, hand edits kept or lost, and kitbashing

### Takeaway
Each of the four cross-cutting questions has exactly one or two opened first-hand data points, and they agree with one another: the damage from a late change comes from dependencies nobody wrote down; layout that must look right is supplied as authored data rather than generated; and hand edits survive when they are anchored to position in the world. Evidence on kitbashing cost is close to nil.

### Cited Findings

A kit piece's contract changes late
- [first-hand] Warhorse: enlarging houses after three quests existed broke the first quest, because characters in it no longer behaved. Lesson stated: in an open world nothing can be finished and locked in isolation. — [interview mirror](https://fextra.pages.dev/posts/kingdom-come-deliverance-interview-with-warhorse-development-update-dlc-taking-chances-building-a-community)
- [first-hand] Embark: shared dimensions (wall thickness, floor height) live once, on the building, and every feature node reads them; a change recooks only dependants; four to six minutes from blockout to finished asset per change. — [SideFX](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)
- [first-hand] Techland: a change to building style or geometry can be pushed across the whole city by the tool. — [Wccftech](https://wccftech.com/the-big-dying-light-2-qa-techland-opens-up-on-c-engine-tech-delays-game-length-much-more/)
- [first-hand] Embark, a contract that was split rather than changed: the Japanese roof got its own node so the general roof contract stayed as it was. — [SideFX](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)

A street or the terrain moves under buildings
- [first-hand, machine-translated] Team Ninja: generated streets were wrong, so the street network became an authored input taken from period maps; blocks and buildings follow from it. — [CGWORLD](https://cgworld.jp/article/202410-cgw313-rotr02.html)
- [first-hand] Ubisoft Montreal on Origins: roads are hand-adjusted splines projected onto terrain; settlements are carved out of procedural vegetation zones and planted by hand so nothing grows through a house. — [80.lv](https://80.lv/articles/creating-the-world-of-assassins-creed-origins/)
- [first-hand] Warhorse on the sequel: terrain from geographic data and village plans from old military maps, so buildings and ground come from sources that already agree. — [Xbox Wire](https://news.xbox.com/en-us/2025/02/04/building-an-authentic-bohemia-in-kingdom-come-deliverance-2/)
- [first-hand documentation] Epic's City Sample: changing lot or zone settings regenerates building volumes and lots wholesale. — [Epic documentation](https://dev.epicgames.com/documentation/unreal-engine/city-sample-quick-start-for-generating-a-city-and-freeway-using-houdini)

Hand edits kept or lost
- [first-hand] Embark: edits are stored in world space as geometry on the tool, not by index into generated geometry, and stay valid while the feature stays in place. — [SideFX](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)
- [first-hand, machine-translated] Team Ninja: after generation, houses were shifted and distorted in response to feedback. The article does not say whether those changes survive a regeneration. — [CGWORLD](https://cgworld.jp/article/202410-cgw313-rotr02.html)
- [first-hand] Ubisoft on Origins: settlements are hand-built from the start, so the question does not arise for buildings; procedural output elsewhere is adjusted by hand. — [GamesBeat](https://gamesbeat.com/how-ubisoft-created-the-art-for-the-massive-world-of-assassins-creed-origins/2)

Kitbashing and reuse outside the design
- [first-hand] Embark reused its Finals building tools for ARC Raiders by dropping the destruction-specific output and turning integrated details into instanced meshes; the cost named is that what one game needs fused into the wall the other wants separate. — [SideFX](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)
- [first-hand] Creative Assembly intends to take interior-style modular environments outdoors for the Alien: Isolation sequel and says it is still considering how they should look there. — [80.lv](https://80.lv/articles/how-alien-isolation-2-expands-on-the-original-s-terrifying-atmosphere)
- [search summary, page not opened] Guerrilla artist: Tenakth decoration building blocks were kitbashed into assemblies for one settlement. No cost stated. — [ArtStation](https://niekschlosser.artstation.com/projects/8wob9E)

Coverage of the six kinds of place (developer accounts found, stated plainly)
- (1) Medieval or fantasy villages: yes, but thin. Warhorse on house size and on layout sources. Nothing on kit structure.
- (2) Modern cities: yes. Embark (rich), Techland, Frogwares (partial), Epic (abstract and documentation).
- (3) Traditional East Asian architecture: yes. Team Ninja on Edo streets and blocks; Embark on the Japanese roof node; Ubisoft on Shadows climbing rules. Nothing on brackets, bays or compounds.
- (4) Industrial sites and infrastructure: no developer account of an industrial exterior kit was opened. Only Creative Assembly's pipeline remark and unopened Satisfactory patch notes.
- (5) Classical or monumental architecture: monumental yes (Notre-Dame in Unity, first-hand). Classical no: the only trace is an unopened portfolio line about a villa kit with column pieces for Odyssey.
- (6) Buildings that adapt to terrain: no developer account found. Not for hill towns, stilts or terraces, in any game searched.

### Inferences
- [inference] In every opened account where layout had to look right (Edo, Bohemia, Egypt's settlements), the layout was authored or surveyed data and the generator worked around it. Where layout was generated (City Sample, CityBuilder), a change regenerates wholesale and the accounts stay silent on hand edits. Embark is the only source that describes hand edits and regeneration together.
- [inference] The Warhorse and Embark accounts are two answers to the same hazard. Warhorse discovered an unwritten dependency on house size after the fact. Embark writes shared dimensions down once and makes every consumer read them. The first is what happens without the second.
- [inference] The absence of terrain-adaptation accounts may itself be a finding: the opened sources either flatten the question (buildings placed on blocks, slope handled by the terrain team) or do not mention it.

### Gaps
- No account found of a module or grid size being changed late in any of these games, nor of pivots or corner pieces being redefined. An academic paper surfaced by search asserts that such changes are expensive; it is not developer testimony and was not opened.
- No account found of what happened to finished buildings when a street was moved under them.
- Evidence on the cost of kitbashing is essentially absent.
- The strongest leads not read, all video or behind a login, in case a later pass can watch them: Insomniac, [Procedural Tool Development for Marvel's Spider-Man 2](https://gdcvault.com/play/1034803/Technical-Artist-Summit-Procedural-Tool) (GDC 2024) and [Ten Years of Tech Art for Marvel's Spider-Man](https://vimeo.com/982767414) (2024); Epic, [The Matrix Awakens: Generating a World](https://www.sidefx.com/learn/talks/the-matrix-awakens-generating-a-world/); Guerrilla, [Designing the Settlements of Horizon Forbidden West](https://www.gdcvault.com/play/1027747/Art-Direction-Summit-Designing-the) (GDC 2022); Hangar 13, [From Destroying Buildings to Procedural Ecosystems](https://www.sidefx.com/learn/talks/from-destroying-buildings-to-procedural-ecosystems/). Nothing is claimed here about their content beyond their published abstracts.
