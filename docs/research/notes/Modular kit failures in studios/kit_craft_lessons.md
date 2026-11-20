# Craft lessons from modular kits and kitbashing: mistakes, causes, fixes, rules

Reading notes for the report writer.

**Source tags used on every finding**

- **[S]** practitioner who shipped games with the kits described (or with comparable kits).
- **[S2]** third-party write-up of a shipped practitioner's talk (secondary, not the speaker's own text).
- **[N]** author with no shipped credit stated on the page, hobbyist, or forum poster with unknown credentials.
- **[V]** vendor documentation, vendor blog or marketplace listing.
- **[SR]** search-result summary only; the page itself was not opened.

**How the pages were read.** Every page was read through a fetch tool that returns a summary by a small model, not the raw page. Numbers and short quotes below are as that tool reported them and should be checked against the source before being quoted in a final report.

**Sources that could not be opened (nothing below is taken from them)**

- Lee Perry, "Modular Level and Component Design" (Game Developer, Nov 2002): the Unreal docs page returned HTTP 403 (https://docs.unrealengine.com/udk/Three/ModularLevelDesign.html), the Wayback copy was blocked by the tool, the UDN PDF redirects to a home page, and the Scribd copy (https://www.scribd.com/document/44212040/Modular-Level-Design) shows only a landing-page blurb.
- Kevin Johnstone, "Modular Environment Design, or how I learned to stop worrying and love the grid" (ECGC 2011 slides): only a forum mention was found (https://lpc.opengameart.org/node/99813); the slides were not located. The link to Unreal Tournament 3 in the brief could not be confirmed.
- Polycount wiki "Modular environments" (http://wiki.polycount.com/wiki/Modular_environments): socket closed on two attempts.
- Joel Burgess's own blog post of the 2013 talk (http://blog.joelburgess.com/2013/04/skyrims-modular-level-design-gdc-2013.html): socket closed. The same talk was read in its Game Developer edition instead.
- Fallout 4 GDC 2016 slides: only slides 1 to 100 of 274 came through (https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4). The later part of the talk is covered only through a Russian third-party write-up, tagged [S2].
- Neil Blevins, "What Is Kitbashing, a Kit, and a Prefab" (http://neilblevins.com/art_lessons/prefab_kit/prefab_kit.htm): certificate expired.
- Tor Frick, Paul Mader, Mapcore threads, and GDC Vault talks from 2018 to 2026: none were opened. A GDC 2026 session "The Modular and Expressive World Art of 'Keeper'" exists in the schedule (https://schedule.gdconf.com/session/the-modular-and-expressive-world-art-of-keeper/915052) and a GDC Vault entry "Building Blocks: Artist Driven Procedural Buildings" lists roofs and non-rectangular faces among its topics (https://gdcvault.com/play/1012655); neither was read.

---

## 1. What "kitbashing" means today, as distinct from a "modular kit", and what goes wrong

### Takeaway
In the best-documented studio usage (Bethesda), a modular kit is a rule system of pieces sharing a footprint, pivot and transitions, and "kit-bashing" (also "kit-jamming", "kit mashup") means mixing pieces from different kits that were not designed together; the named failures are bad intersections and lighting issues, and the fix is to plan for mixing up front (shared door standard, glue kits, sockets and plugs). The other sense, bashing parts into new objects, shows up in Fallout 4's mechanical parts kits; the specific failures the brief asked about (collision, navigation, overdraw, lost swappability) were not found described in any practitioner source opened.

### Cited Findings
- [S] A kit is described as a system, not an asset list: a four-piece pipe kit yields unlimited configurations, and the kit "adds up to far more than the sum of its parts". — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Kit-bashing is defined as mixing kits, e.g. a Dwemer hallway combined with ice caves for an instant shift of tone. Named risks: bad intersections and lighting issues. Rule drawn: artists should allow the experimentation and design kits with mixing in mind. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S, journalist paraphrase of Burgess] Same distinction restated: a modular kit gives reuse inside one system (the pipe example is four shapes: straight, T, curve, end); kitbashing mixes systems to break up that reuse (ruins and vines inside cave kits, dwarven halls fused with ice caverns). — [PCGamesN, Jeremy Peel](https://www.pcgamesn.com/fallout-4/bethesda-open-world-design)
- [S] Difficulty: kits with unrelated logic cannot be joined. Fix: one uniform door-frame dimension across all kits, so kits transition into each other and doors are reusable, and AI and animation get one standard. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] The trade-off is stated as a catch-22: shared logic across kits improves compatibility but makes repetition more obvious. Rule: use similar logic for kits that replace or combine with each other (temple and dungeon; pristine and destroyed station), distinct logic for isolated environments; keep one portal standard for doors, windows and other openings regardless. — [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)
- [S] "Glue kits" are small kits made on purpose for transitions between kits and sub-kits and built with kit-bashing in mind (platforms, tunnels, ledges). The studio's later project used them heavily. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] Fallout 4 "sockets and plugs": standard openings (sockets) are paired with simple connectors (plugs) that work with any kit on the other side, so no bespoke transition model is needed per kit pair. — [DTF write-up of the GDC 2016 talk](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] Anti-example: an asymmetric one-way "utility" kit for basement pipes cost too much and was barely used as designed; level designers took just its pipes and used them with different steps and joining rules as glue between other kits. — [DTF write-up of the GDC 2016 talk](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] Bashing parts into new things: mechanical details (rivets, tanks, fasteners) were built as kits and assembled into machines, vending machines and cars; about 400 cars are reported made this way, against about two weeks of concept and modelling for one unique asset. — [DTF write-up of the GDC 2016 talk](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] Some Fallout 4 locations combined 7 to 8 kits, each with its own textures. — [DTF write-up of the GDC 2016 talk](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Unintended use is expected, not an edge case: level designers built pillars out of hallway corners and 3-way junctions; "Somebody always does". Rule: raise unintended uses early and decide jointly what the kit supports and does not, and write it down. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Shell-based building as a controlled form of bashing: grid-snapped shell pieces, then free-placed walls, pillars and balconies inside; awkward intersections are caught in art review and hidden with ambient occlusion. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Cost named for heavy modularisation in a shipped multiplayer game: more scene assets, which can raise draw calls and collision counts; to be balanced case by case. — [Carlos Sánchez, PUBG, 80.lv 2021](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [SR] Generic definitions (not game-specific): kitbashing as assembling new 3D assets from pre-made parts by combining, scaling and rearranging; stated risks are repetitive results from one kit and style clashes when mixing kits; one glossary says the approved kitbash is usually retopologised into a clean mesh afterwards. — [Wikipedia: Kitbashing](https://en.wikipedia.org/wiki/Kitbashing); [nhance glossary](https://nhance-school.com/tools/glossary/kitbashing)

### Inferences
- Two different contracts hide under one word. Mixing kits needs a contract between kits (portal standard, plugs, glue kit). Bashing parts into objects needs almost no contract but leaves the result unswappable; the Fallout 4 answer was to bake the bashed result (pack-in or merged mesh) so it becomes one piece again. This reading is mine; the sources do not state it in these words.
- The utility-kit anti-example suggests that when a kit's rules are too costly, users strip it down to its most rule-free pieces. A tool could treat that as a signal that a kit's contract is over-specified.

### Gaps
- No practitioner source opened describes kitbashing failures in collision, navigation mesh, overdraw, or lost swappability. The only named failures are intersections, lighting, draw calls and collision counts.
- Nothing was found on kitbashing with dense-geometry pipelines (for example Nanite) and overlapping meshes; one search returned no relevant results.

---

## 2. Planning a kit: early decisions practitioners say they got wrong, and the cost of changing them

### Takeaway
The decisions that cannot be changed cheaply are the footprint, the pivot, the snap rules and the transition (door) metrics; every source that discusses revision names these. The wall-on-grid-line question has no free answer: centring the wall on the line cuts the piece count but eats room size and breaks interior metrics.

### Cited Findings

**Footprint and module size**
- [S] Mistake: sub-kit footprints that are not multiples of each other. A 512 room tiles with a 256 hallway; a 384 room creates gaps or overlaps. Rule: sub-kit footprints are multiples of one base. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Mistake: treating the footprint as walkable space. A hallway alcove that sticks outside the footprint blocks the neighbouring hallway. Rule: the footprint is the full bounds; everything, alcoves included, sits inside it. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); restated as fundamentals "Begin by defining the footprint" and "Build within footprint extents" in [Burgess and Purkeypile, GDC 2016 slides 25 to 40](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [S] Rule: set the snap grid as large as possible, typically half the footprint, so misalignment is visible at once. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Mistake: trying to tile on all axes. Rule: hallways tile on one axis, rooms on two; vertical stacking gets its own sub-kit with a fixed horizontal footprint (the cave and Nordic "shaft"). GDC 2016 restates it as "Be cautious with non-uniform tiling". — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [S] Equilateral footprints are the most versatile and the most repetitive-looking; the choice follows priorities, scale and proportion. — [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)
- [Klafke; shipped credits not stated on the page] Mistake: an off-ratio piece (192x128 in a 128 system). Rule: one base size for the whole set (128x128 for 1x1, so 2x1 is 256x128); power-of-two grid steps so halving stays on grid (8, 4, 2, 1 against 5, 2.5, 1.25). Piece size follows player and space scale: small for corridors, large for urban facades; pieces outside the playable area can be larger and merged. — [Thiago Klafke, modular environments](https://www.thiagoklafke.com/modularenvironments.html)
- [N] Same halving argument from a different author: a 4 m main piece splits to 2 m and 1 m and grows to 8 m; a 5 m piece gives 2.5 m parts and snapping trouble. — [Lea Kronenberger, Beyond Extent 2020](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)
- [N] Mistake: mixed wall lengths (2 m and 2.5 m) with a 100-unit snap, forcing every user to change snap settings. Fix offered by forum posters: drop to a 5, 10, 25 or 50 cm snap. Note that this fix contradicts Burgess's rule of the largest possible snap. — [Polycount thread, UE4 Modular Building Set Breakdown](https://polycount.com/discussion/comment/2661655)
- [V] A vendor kit sets the minimum piece length (7.5 studs) and makes all longer lengths divisible by it, so pieces still align after rotation; move snap equals that length, rotate snap is 45 degrees. — [Roblox docs, Assembling modular environments](https://create.roblox.com/docs/tutorials/3D-art/assembling-modular-environments)

**Wall thickness and where the wall sits relative to the grid line**
- [N] Wall centre-line on the grid line needs fewer unique modules and has fewer edge cases, but the wall then eats into the room: on a 1 m grid a 4 m clear space between walls can no longer be laid out on the grid. The poster says he never found a project where exact interior dimensions did not matter. Otherwise the wall must be covered on both sides of the line, and "there'll be quite a lot of parts". — [Polycount, "Creating modular walls with thickness", poster poopipe](https://polycount.com/discussion/232991/creating-modular-walls-with-thickness-how-to-handle-corners-and-alignment)
- [N] A hobbyist's worked answer: 3 m cubic footprint, 1.5 m snap, 0.25 m subgrid, walls 0.125 m thick sitting 0.125 m inside the footprint edge (planes at plus or minus 1.375 m from centre). Reason given: 0.25 m walls back to back make 0.5 m, which looks too thick; zero-thickness walls z-fight back to back and show knife edges. — [Ian London, Modular Level Kit Design Considerations](https://ianlondon.github.io/posts/modular-level-kit-geometry)
- [N] Check during blockout whether parts need thickness for placement at all. — [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)

**Pivot**
- [S] Pivots are usually at the ground plane, centred; exceptions are functional (a pipe pivoting at its edge like an arm, a platform at its base). Changing a pivot later means manual updates to hundreds of instances. GDC 2016 fundamental: "Pick a pivot and stick with it". — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [Klafke] Opposite convention: pivot at one extremity of the piece's plane, because centred pivots make scaling and rotating harder; the same pivot position for every piece of a wall set, so units swap in place in the editor. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)
- [N] Lower-left corner by default; but for walls with thickness a centred pivot lets one corner piece serve as both inside and outside corner. — [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)
- [N] Origin at the centre of the footprint box, not of the mesh, so a piece rotates without translating. Self-check: rotate 90 degrees on each axis and confirm it is still on grid. Small attached details (switches, lamps) take their origin on the surface they attach to. — [Ian London](https://ianlondon.github.io/posts/modular-level-kit-geometry)
- [V] A 10 x 10 x 1 part with a centred pivot misaligns after rotation (0.5 stud overlap per side). The vendor kit puts pivots at the forward-most lower corner. Moving several selected pieces together averages their pivots and destroys the snap. — [Roblox docs](https://create.roblox.com/docs/tutorials/3D-art/assembling-modular-environments)

**Storey height, doors and windows**
- [S] Metrics set before any kit: character about 128 units (about six feet); one door-frame size for every kit; narrowest walkable space two character widths so two AI can pass; AI can climb about 60 degrees but kits should use 30 to 45 because steep slopes animate badly; jump, fall and cover heights where the game needs them. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] Fallout 4 standardised single and double door-frame sizes across all kits, known to the whole team; this is what lets a concrete basement join a brick wall. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] In a shipped shooter, interior layout and window placement follow strict gameplay rules (a clear, fast looting path), so the kit serves those rules, not the reverse. — [Sánchez, PUBG, 80.lv](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [N] A 3 m cube was chosen because it fits a doorway and sets ceiling height for a 2 m character; holes for doors and windows run to the footprint edge so the inner frame matches the neighbour; left-right symmetric interfaces let pieces combine in more orientations; details stay inset from the edge so neighbours do not clip. — [Ian London](https://ianlondon.github.io/posts/modular-level-kit-geometry)

**Walls, rooms or whole facades** (see also section 7)
- [S] The first question is purpose: "What problem does a kit solve?"; each kit carries overhead in lead time, on-boarding, iteration and maintenance, so like kits were consolidated. — [GDC 2016 slides 70 to 75](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)

**Cost of changing later**
- [S] Changing pivot location, base footprint size or transition metrics after levels are built can break hundreds or thousands of layouts. Artists often do not see why moving a tiling edge matters and need mentoring; they must accept early commitment to proportions fixed by kit dimensions. — [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)
- [S] Scope amplifies errors: mistakes "scale to weeks and months". — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Problems in one shipped project's kit came mostly from meshes added late that had not been planned and forced a rethink. Rule: list the meshes from reference up front and test in engine before committing. — [Vuk Banovic, exp-points](https://www.exp-points.com/vuk-single-material-modular-kit-environment-ue4)

### Inferences
- Sources disagree on the pivot (centred at ground, edge, lower-left corner, footprint centre) but agree on the rule above it: one convention per kit, never changed, and identical among pieces meant to swap. The convention matters less than its constancy.
- The wall-on-line decision is really a choice of which quantity stays on the grid: the structure (centre-line) or the clear room size (face). No source offers a way to have both without extra pieces.

### Gaps
- No practitioner account of choosing a storey height and regretting it was found.
- No source gives a measured cost (days, levels rebuilt) for a late pivot or footprint change; the cost is stated only as "hundreds or thousands" of instances or layouts.
- Lee Perry's and Kevin Johnstone's texts, the likeliest sources for early Epic decisions, could not be opened.

---

## 3. Corners

### Takeaway
Thick walls make corners a separate problem: an inside corner overlaps itself and an outside corner leaves a notch, so both need their own pieces or a cover piece (pillar, bevel, cap). Off-grid angles are handled by covering the gap with a flange or pillar, never by making the pieces meet.

### Cited Findings
- [N] Mistake: lining thick walls up directly at a corner. Cause: with the wall inset from the grid line, an inside corner overlaps itself on a full-width wall and an outside corner leaves a gap. Fix: inside corner as its own part; bevels or caps for outside corners. — [Ian London](https://ianlondon.github.io/posts/modular-level-kit-geometry)
- [N] The two options an artist faced with real brick-geometry walls: corner pieces that include the thickness plus connector blocks between modules (leaves padding he found limiting), or walls in direct contact that must overlap at corners. Both gave inconsistencies at some corners. Advice received: make corner pieces per angle, plus 3-way and 4-way junction pieces; test with S-shaped layouts; then cut the modules up to find the truly unique parts. — [Polycount, "Creating modular walls with thickness"](https://polycount.com/discussion/232991/creating-modular-walls-with-thickness-how-to-handle-corners-and-alignment)
- [N] A centred pivot on a thick wall lets one corner part act as both inner and outer corner. — [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)
- [S2] Fallout 4 "layered inserts": right-angle corners are hidden with floor-to-ceiling inserts, and the inserts' seams are masked with columns. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4); listed as technique 1 in [GDC 2016 slides 42 to 53](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [S] Mistake found late in a shipped season: buildings assembled from modules inside the engine had bare, hard corners, because smoothing across module boundaries is not possible once pieces are separate meshes. Fix: about half of the building-creation time went into small supplementary assets, chiefly brick corner pieces (also reused around doors and windows), a sculpted stone base, and roof pieces. — [Sánchez, PUBG, 80.lv](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [S] An extra pillar is an accepted way to solve a join; the author otherwise keeps to the kit's grid step (10 units) even for decorative pieces. — [Vuk Banovic, exp-points](https://www.exp-points.com/vuk-single-material-modular-kit-environment-ue4)
- [S] Loopback failure caused by a corner: a 2-way corner with an uneven stub left a door half a footprint off when the hallway looped back. The tempting fix, a half-length "patch-up" hallway, bloats the kit and adds arbitrary rules. Rule: fix the core piece. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Non-90-degree corners: with snap-to-reference a rotated piece becomes the new grid origin, and gaps appear where differently angled groups meet. "Pivot and flange" (Skyrim's Ratway) lets hallway pieces turn at any angle with a full-curvature archway covering the gap. Costs: layouts drift so loops no longer close, tiny holes are hard to spot (found by playing often), and it suits decayed tunnels, not clean architecture. Rule: bend one logic quirk per kit; doing it everywhere spirals. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] Fallout 4 used a local grid attached to a parent object (configurable step and rotation) to build at angles, and flexible pieces for curved streets in Boston. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [N] For 90 and 45 degrees a bevel part is better; pivot-and-flange is for odd angles. Angled wall groups look more natural and the gaps can be filled with pillars or fill blocks. — [Ian London](https://ianlondon.github.io/posts/modular-level-kit-geometry); [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)
- [Klafke] Curved walls: every bounding box, 90-degree curves included, must stay in a 1x1 ratio of the base unit; contact points are left untouched. A bend deformer leaves end vertices off grid (for example at -639.99 instead of -640) and always needs the ends snapped by hand. He classes pieces by contact: edges on the pivot plane combine freely; pieces whose edges do not reach it combine only with their own kind or need fillers. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)
- [S] Directional kits (Fallout 3): pieces that cannot rotate freely allow asymmetric halls, but when an asymmetric hallway loops back it arrives flipped, and a "de-twist" piece is needed to swap sides within one piece length. Judged usually not worth it for architectural kits. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)

### Inferences
- Corner pieces are where the wall-on-line decision is paid for. A kit whose walls sit on one side of the line needs at least inside corner, outside corner, T and cross variants per wall type; a centred wall can reuse one.
- A pillar or flange is a piece whose contract is "covers a join of unknown angle"; it trades exactness for tolerance and is the cheapest cure the sources name.

### Gaps
- No source measured how many corner variants a real shipped building kit carried.
- Curved exterior walls of buildings (as opposed to curved corridors) are not discussed in any source opened.

---

## 4. Stacking: floor thickness, stairs, overhangs, balconies, trim between storeys

### Takeaway
The repeated mistake is a floor with no thickness: stacked pieces become co-planar and z-fight. Floor thickness has to be fixed with the footprint, and vertical stacking is treated as its own sub-kit or layer set. Stairs are the piece that fits the module worst.

### Cited Findings
- [S] Mistake: stacking hallways gave co-planar floor and ceiling. Effects: no visible thickness, z-fighting flicker, harder optimisation and trigger placement, a confusing editor view. Rule: decide floor thickness in advance. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [Level Design Book, citing Burgess] The "stack test": can the kit build multi-level spaces; floors should have thickness and mass like walls, not be paper thin. — [The Level Design Book, Modular kit design](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- [S] Vertical stacking is given its own sub-kit with a fixed horizontal footprint. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] Fallout 4's exterior "deco" building kit is built in layers: foundation, lobby or ground floor, awning, repeating floor layer, roof. Interiors are enterable without a loading screen, which forces the facade layers and the interior to agree. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] In the Fallout 4 building kit, pieces are allowed to overlap directly when stacking, which avoided complicated naming and ordering conventions. Walls, floors and ceilings are separate pieces and separate material slots. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] Broken floors: holes and floating platforms with ramps; platforms raise and lower independently and whole floors can be removed. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] The building kit's standard sub-kits are rooms, halls, stairs, railings and platforms, in small, medium and large cell types. — [GDC 2016 slides 77 to 79](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [N] Stairs do not fit a footprint cleanly; the author offset the stair footprint by half its height (1.5 m) to meet floating floor panels and says the arrangement may be a bad design. Balcony-like floors with a top and a bottom are centred between planes; independent floors and ceilings are offset 0.125 m from footprint top and bottom. Tall pillars are a repeating middle tile with end caps, not one piece per height. — [Ian London](https://ianlondon.github.io/posts/modular-level-kit-geometry)
- [N] A staircase modelled for one connection will not fit elsewhere; split into start, middle and end pieces it gives any length, and the same split works for curved stairs. — [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)
- [V] A vendor city kit sets pivots so that balconies, cornices and awnings snap logically to the wall pieces. — [Roblox docs](https://create.roblox.com/docs/tutorials/3D-art/assembling-modular-environments)

### Inferences
- "Awning" as a named layer between ground floor and upper floors in the Fallout 4 facade stack is the nearest thing found to a trim-between-storeys rule: the horizontal band is its own layer and hides the join between two different floor types.
- Allowing overlap when stacking is a deliberate loosening of the contract to save naming and ordering rules; it depends on the overlapped parts being hidden.

### Gaps
- No practitioner account of overhanging upper storeys (jetties) or of balcony pieces going wrong was found.
- No source gives stair rise-and-run numbers tied to a storey height.

---

## 5. Roofs

### Takeaway
This is the thinnest part of the evidence. No shipped practitioner account of pitched-roof kits on L-shaped or irregular plans was found. What exists: a forum rule of thumb (treat a roof like a hallway kit: middle, inside corner, outside corner, end cap), marketplace listings showing roofs as the largest share of a house kit, and shipped teams that treat the roof as the swappable part or as a last-minute supplementary asset.

### Cited Findings
- [N] Rule of thumb from an Unreal forum reply: build a roof like a hallway kit with four pieces, a middle section, an inner corner, an outer corner and an end cap. The same reply argues roofs are rarely seen closely, so plausibility is enough. — [Unreal Engine forums, "Designing modular roofs?"](https://forums.unrealengine.com/t/designing-modular-roofs/156062/2)
- [S] Roofs were among the supplementary assets a shipped team had to add after assembling modular buildings (a teammate made roof pieces alongside corner pieces and a stone base). Before that season, roofs and trims were the parts of buildings that were already modular. The team also reuses one building base mesh and changes roof, materials and set dressing to make a different-looking building. — [Sánchez, PUBG, 80.lv](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [S2] In Fallout 4's layered facade kit the roof is the top layer of the stack. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] A reading-room ceiling and windows in a library could not be made from standard pieces; a list written up front had missed them and the need only showed through iterative building. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Corners, roofs and foundations are named as the tricky parts to model in a building set, and the reason beginners should practise on a simple tileable building. — [Vuk Banovic, exp-points](https://www.exp-points.com/vuk-single-material-modular-kit-environment-ue4)
- [V, SR] Piece-count evidence from marketplace listings: one low-poly building pack lists 20 roof parts among 44 pieces; a Japanese shrine roof alone is sold as 18 meshes; a tabletop roof set divides into surfaces, valleys, eaves in two orientations, and ridges and hips; a medieval house add-on lists 10+ roof and chimney pieces and 4 gable pieces beside 16 wall pieces. — [itch.io Low Poly Building Kit](https://mb-studio0.itch.io/low-poly-building-kit); [itch.io Modular Japanese Shrine Roof](https://darkcomdev.itch.io/modular-japanese-shrine-roof); [2-Minute Tabletop rooftop assets](https://2minutetabletop.com/modular-rooftop-map-assets/); [Modular Medieval House Creator](https://superhivemarket.com/products/modular-medieval-house-creator)

### Inferences
- Roof kits balloon because a pitched roof has more distinct join conditions than a wall: slope, ridge, eave, gable end, inside (valley) and outside (hip) corner, each possibly at several spans and pitches. The listings are consistent with this but no practitioner states it; the explanation is mine.
- The shipped pattern visible in the sources is avoidance: flat or simple roofs, a roof swapped as a single large piece per building base, or roofs added by hand late. That suggests roofs are commonly handled at a coarser granularity than walls.

### Gaps
- Nothing found on ridge, valley and hip pieces for L-shaped plans, on eaves, gable ends, dormers or chimneys as kit pieces, or on how artists cut the piece count, from anyone who shipped a game.
- The GDC Vault talk "Building Blocks: Artist Driven Procedural Buildings" lists roofs and non-rectangular faces among its topics (https://gdcvault.com/play/1012655) but was not opened.

---

## 6. Meeting the ground: foundations, slopes, steps, terraces, stilts

### Takeaway
Almost nothing was found. The only shipped evidence is that a foundation layer and a stone base exist as separate pieces under the wall modules, and that platform or glue kits are what sit on other surfaces.

### Cited Findings
- [S2] Fallout 4's facade stack starts with a foundation layer below the ground-floor layer. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] A sculpted stone base was one of the supplementary assets a shipped team added under its modular buildings. — [Sánchez, PUBG, 80.lv](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [S] Platform kits (a kind of glue kit) are made to sit on other surfaces, cut up horizontal play planes, and were reused as exterior points of interest. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Slope metric that bears on steps and ramps: AI climbs about 60 degrees but kits should use 30 to 45. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] City streets: Fallout 4's Boston first tried large 3D buildings, which failed and cost about a year of schedule; the fix was much smaller house modules than Fallout 3, more designers, and flexible pieces for curved streets. Flat streets were also named as a problem for AI and combat. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S, personal project] One artist blocked out street layout and scale with primitives and did basic landscape sculpting before or alongside placing modular houses. — [Sourav Kumar, 80.lv](https://80.lv/articles/medieval-village-production-guide)

### Inferences
- A tall foundation or base piece sunk into terrain appears to be the default answer to slopes, since the base is a separate piece in both shipped examples; no source says so directly.

### Gaps
- No practitioner writing found on foundation skirts on slopes, steps up to doors, terraced or stilt buildings, or pieces sunk into terrain. A targeted search returned only real-world construction pages and marketplace listings.

---

## 7. Granularity: small pieces, large pieces, and prefab or pack-in assemblies between

### Takeaway
Bethesda moved from merged room-sized meshes to separate wall, floor and ceiling pieces grouped into editor prefabs ("pack-ins") that can be exploded when a part must change; the cost was a sixfold rise in object count for the same room. Large whole-building pieces failed for their city. Other shipped teams go the other way for distant or repeated buildings and merge.

### Cited Findings
- [S] Old way: room and hall chunks exported whole from the modelling tool, with much redundant work. New way: granular pieces (hall, floor-only, wall, wall-corner, ceiling-only) assembled in the editor into pack-ins. Stated gains: wall, floor and ceiling can be grouped and regrouped, more freedom for destruction, pieces interchange between kits, layouts no longer depend on predetermined combinations. Stated cost: more time, and one example room goes from 20 objects to 123. — [GDC 2016 slides 81 to 96](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [S2] The pack-in answers the object count: pieces are packed into prefabs, the prefab can be broken apart to swap a part (for example a damaged wall), and prefabs are baked into single meshes for optimisation. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] Whole large 3D buildings failed for Boston and were replaced by much smaller house modules. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Oblivion's "warehouse" cells (pre-lit, pre-cluttered rooms copied and pasted) were the coarsest level and produced the "cookie-cutter" complaint; from Fallout 3 on the studio built from the most granular art available. Players notice repeated clutter before repeated architecture. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [Level Design Book] The choice is stated plainly: finer pieces allow more customisation, chunkier pieces allow faster prototyping; decide in pre-production. — [The Level Design Book](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- [S] Hero pieces are one-offs used a few times while standard pieces are used hundreds or thousands of times. Fallout 4's order of work: utility pieces (walls, stairs), then variants (material, damage), then hero pieces, late because they are expensive. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] In a shipped multiplayer game: shared pieces instance across the map and are usually cheaper, but too many small modules raise draw calls and collision counts; one building base mesh is recycled with a different roof and dressing. — [Sánchez, PUBG, 80.lv](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [S] For a facade, the unit of modularity is found by looking for repeating structural elements (windows, pillars). By the vista stage whole buildings already exist and only their distant versions are made. Modularity is justified by schedule and memory. — [Mathieu Gasperin, Arkane, 80.lv 2017](https://80.lv/articles/modular-design-in-dishonored-2)
- [N] Pre-combine whole buildings or background houses in the modelling tool, or merge in engine; use an engine prefab to duplicate assembled groups. Example of splitting for variety: three whole broken-window variants against one cut-out wall, two frames and six glass pieces. Balance large and small pieces; too many small ones read as clutter. — [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)
- [N] A four-level naming of granularity: template proofs (greybox), atoms (details that never leave the modelling tool), molecules (exported pieces a level designer uses), compounds (groups of molecules, engine prefabs). — [Ian London](https://ianlondon.github.io/posts/modular-level-kit-geometry)
- [Klafke] Mix levels by distance: small modules where the player is close (pavement), large ones for facades, merged ones outside the playable area. Modularity suits sci-fi and urban best; organic parts are better unique, still placed on the grid. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)

### Inferences
- The pack-in is the piece of this craft closest to a tool feature: a named assembly that behaves as one piece for placement and cost, yet keeps its parts addressable for swapping and damage.
- Sources agree that the right level differs by distance from the player and by how often a combination recurs; none gives a rule for deciding other than use.

### Gaps
- No source describes problems caused by mixing levels in one level (for example a pack-in beside loose pieces going out of date after a kit change).
- How pack-ins are kept in sync when a constituent piece changes is not described in the slides that were readable.

---

## 8. Variation and repetition: shared footprint and pivot, damage, kit size, glue and filler

### Takeaway
Variants are safe only when they keep footprint, pivot and role, which is what lets a tool or a scroll wheel swap them; damage is best split into independently varied halves. Kit size follows how often the kit is used, and "patch-up" pieces that paper over a logic error are the named cause of bloat.

### Cited Findings
- [S] Variants are pieces with the same footprint and role, named with a 01/02/03 suffix so they sort together. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [Klafke] The same pivot position across a wall set is what makes units swappable in the editor. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)
- [S2] Fallout 4 editor support for swapping: holding a key and scrolling swaps a piece for a related one; another combination swaps its material. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [Level Design Book] Do not make variants until the basic pieces work. Texture variants are cheap because the geometry need not be re-tested. A tool that swaps module types speeds the art pass. — [The Level Design Book](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- [S] Build one visually complete piece before any variants, so an art-direction change costs one piece and not the kit. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] Damage: repeated damage looks artificial. Eleven wall-hole variants varied independently on each side give 121 combinations, and damage pieces can be mixed across kits. One geometry set carries many materials (brick, wallpaper, wood panel, concrete and others); adding the variants took about two weeks, mostly for damage. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Kit size follows use: Skyrim's cave kit was used over 200 times and has seven sub-kits, with about 50 pieces in a small hallway sub-kit; the Ratway kit was used twice, has three sub-kits and seven hall pieces. Two kit artists and eight level designers produced seven kits and over 400 cells in about two and a half years. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] Fallout 4 planned about 14 kit families, estimated player time per family from Fallout 3 data, then merged many into one building kit, which ended up used on nearly every level, more than planned. Internal kits had about 500 pieces against under 100 in the player-facing workshop. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4); kit list on [GDC 2016 slide 68](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [S] Bloat: "It is very easy for an artist to say 'yes' to every request." Rule: choose battles. Patch pieces subvert the kit's logic and harm it in the long run. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)
- [S] Naming failure: a Fallout 3 name such as `UtlBayCorInMidPRTT01L01` held a field (PRTT01) meant to encode which neighbours a piece tiles with; nobody could internalise it and the kit was underused. Rule: names readable by someone who has never seen the kit; hallway pieces named by exits (1Way, 2Way, 3Way, 4Way); the same words across all kits. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Art fatigue: reuse erodes the world's authenticity. Fixes named: interoperable variants and props, mixing kits, and not tying a kit to one inhabitant, lighting or gameplay type. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)
- [Klafke] Filler pieces are the price of pieces whose edges do not reach the pivot plane; some combine horizontally but need floor and ceiling fillers vertically. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)
- [Level Design Book, citing Burgess] The "gap test": does an interior kit have enough glue for off-angle construction; essential for tunnel and cave kits. — [The Level Design Book](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- [S] Four brick pieces from one trim sheet made a 400 x 400 unit wall without visible repetition; highly detailed pieces should not sit next to equally detailed ones. — [Vuk Banovic, exp-points](https://www.exp-points.com/vuk-single-material-modular-kit-environment-ue4)
- [N] Ledges, panels and single bricks placed freely are a cheap source of variation; decals and world-space shader variation break the kit look without new geometry. — [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)
- [V, SR] Vendor blog claims, unverified: a 12-piece kit that is maintained beats a 60-piece kit with three pivot conventions; kits fail when modularity is imposed after "cool pieces" already exist. — [Stray Spark blog](https://www.strayspark.studio/blog/modular-kit-workflow-greybox-to-final-ue5)

### Inferences
- The failed PRTT field is direct evidence that adjacency rules encoded in names do not work for people; if adjacency is to be a rule, it has to be carried by the tool.
- Splitting a damaged wall into two independently varied halves is a general trick: variation multiplies when a piece is divided along a line that the contract does not care about.

### Gaps
- No source gives a minimum piece count for a usable building kit as a general figure; the numbers above are per kit and per studio.

---

## 9. Testing a kit before build-out: tests actually run, and what slipped through

### Takeaway
Bethesda's named tests are loopback, stack, gap and the "what if someone builds a pillared room" misuse test, run on untextured proof pieces by a level designer paired with the kit artist. What slipped through elsewhere: unplanned late pieces, bare corners after in-engine assembly, hero spaces the piece list missed, and handoff changes that silently removed gameplay function.

### Cited Findings
- [S] Stages with durations: concept; proof (1 to 3 weeks; almost no mesh detail, tests proportions, kit logic and naming); graybox (1 to 4 weeks; start with the most-used sub-kit so the level designer stress-tests with real layout); build-out (months); polish (ongoing). — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Loopback test: tile the kit back on itself; it exposed a door half a footprint off, caused by an uneven corner stub. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Stack test: exposed co-planar floors and ceilings. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Gap test: rotated groups leave gaps because rigid grids do not align under snap-to-reference. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Misuse test: the level designer should "anticipate likely (and unlikely) use cases" and not test only as intended; the stress test can feel combative. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] A graybox that is too generic "does not actually solve any problems": elements that must tile across footprints (a central pipe running through a room kit) have to be roughed in. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [Level Design Book] Do not stop at the ideal case; collision is one of the biggest issues, check players cannot get stuck. — [The Level Design Book](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- [S] Fallout 4 milestones for the first pass were "Core Kit Is Usable" and "Layout Is Reliable"; the goal named for kits is being "trustworthy" in function while visual iteration continues through development. — [GDC 2016 slides 97 to 100](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- [S2] The studio had no primitive blockout tools, so kits themselves start as grey models with correct proportions and names. An older kit (the Fallout 3 office kit) stood in as placeholder so levels stayed playable. Concept art for the industrial kit was too smooth to tile and was rebuilt as grey prototypes fitted to the grid. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Test in engine early and walk it as the player, not with a free camera; dimension errors hurt both look and play. — [Vuk Banovic, exp-points](https://www.exp-points.com/vuk-single-material-modular-kit-environment-ue4)
- [Klafke] Pieces that look good alone can fail when tiled; export early and test with simple structures. Verify bounds with cube primitives. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)
- [N] S-shaped layouts are a good test for thick-wall corner sets; set the grid to the module size so errors show. — [Polycount, "Creating modular walls with thickness"](https://polycount.com/discussion/232991/creating-modular-walls-with-thickness-how-to-handle-corners-and-alignment)
- [S] Slipped through (handoff): a cover strip became poles too thin to hide behind, a visual blocker became see-through chain-link, bridge supports blocked planned sightlines. Cause: art polishing a level designed without them. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Slipped through: tiny holes in pivot-and-flange layouts, found only by playing the level often. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Slipped through: bare corners in a mostly modular season of buildings, found after assembly and fixed with corner pieces at the cost of about half the building time. — [Sánchez, PUBG, 80.lv](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [S2] Slipped through: special rooms (a library reading room) missing from the up-front piece list. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Helper markers: editor-only marks for snap points, doorways and ceilings to make pieces easier to select and place. Shown as a mock-up in 2013 and, per the write-up of the 2016 talk, implemented later as snap-point hints. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)

### Inferences
- Loopback, stack, gap and S-shape are all closure tests: build a shape that must return to its start and see whether it does. They can be run mechanically on footprints without art.
- Every item that slipped through was either a join condition never assembled in the test (corners after assembly, rotated joins) or a function that was never written into the piece (cover height, sight blocking).

### Gaps
- No account of automated kit tests in a studio was found; all tests described are done by hand by a level designer.
- The Level Design Book page shows the loopback test only as an image, with no text.

---

## 10. Revising a kit after levels are built from it: what breaks

### Takeaway
Instancing makes visual updates free and structural updates dangerous: a new export shows up in every level at once, so a change to footprint, snap rule, pivot or a tiling edge breaks every placement, and even an "obvious fix" must be checked everywhere.

### Cited Findings
- [S] Fix ripple: a fix to one piece can break hundreds of instances, so obvious fixes must be checked everywhere. The same mechanism lets art be polished across the game with no test area ("fly-by deployment of new art"). — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] The risky changes are footprint, snap rules and pivots; pivot changes late mean hand-updating hundreds of instances. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Restated five years on: pivot location, base footprint size and transition metrics can break hundreds or thousands of layouts. Rule: functional polish first so levels can be built, visual polish later when it can be judged in use. — [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)
- [S] The human cause: artists may not understand why changing a tiling edge matters. The kit artist has to hold the whole rule system in their head for the length of the project, and people who can are rare (called "a unicorn"). — [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue); [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S] Polish-phase rule: respond to real use, expect course corrections because many level designers will use the kit in unexpected ways, and refuse requests that bloat the kit. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] A few complex kits can cost less than many simple ones, because each kit needs artists who understand its logic; early playable first passes matter most since late changes are expensive. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Modular pieces reduce rework when a problem appears, since one fix reaches every placement. — [Gasperin, Arkane, 80.lv](https://80.lv/articles/modular-design-in-dishonored-2)
- [N] Leave room in the plan for future extensions of the set; build main parts first and extend. — [Kronenberger, Beyond Extent](https://www.beyondextent.com/articles/balancing-modularity-and-uniqueness-in-environment-art)

### Inferences
- The sources separate two classes of change without naming them: changes inside the footprint and away from its edges (safe, deployed everywhere at once) and changes to the footprint, pivot, edge profile or transition (breaking). A tool could make that split explicit and refuse or flag the second class.

### Gaps
- No source describes a migration method for a kit whose pivot or footprint had to change after build-out, only that it is done by hand.
- No source describes versioning of kits.

---

## 11. Style-specific notes

### Takeaway
Little practitioner writing on the structure of style-specific kits was found. The usable material covers industrial pipe kits, modern city blocks and decayed organic tunnels; timber-frame, East Asian, classical and hillside or stilt kits are gaps.

### Cited Findings

**Industrial pipe and gantry**
- [S] The pipe kit is the model of a kit: four pieces, unlimited configurations. Pipes take an edge pivot so they swing like an arm. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- [S2] Fallout 4's industrial kit: cooling towers and pipes stack by height and come in several thicknesses; some large pipes are walkable; it mixed well with the older steam-tunnel kit. In the steam-tunnel kit the floor level is not fixed in the module, so spaces vary from narrow to wide, and beams and bridges can be pushed into walls. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S2] The asymmetric one-way utility pipe kit was the stated failure; its pipes survived as glue. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)

**Modern city blocks**
- [S2] Layered facade stack (foundation, ground floor or lobby, awning, repeating floor, roof); enterable interiors without a load; whole-building pieces failed and smaller house modules replaced them; curved streets needed flexible pieces; districts suffered from low point-of-interest density and loss of a unified look. — [DTF write-up](https://dtf.ru/gamedev/3788796-modulnyi-dizain-fallout-4)
- [S] Buildings kept simple and recognisable for gameplay; about 80% modular in one season; corners, base and roof needed supplementary pieces. — [Sánchez, PUBG, 80.lv](https://80.lv/articles/creating-diverse-buildings-for-pubg/)
- [V] A vendor city kit: corner pivots, a single base length with all others divisible by it, 45-degree rotation snap. — [Roblox docs](https://create.roblox.com/docs/tutorials/3D-art/assembling-modular-environments)
- [Klafke] Small modules for pavement, large for facades. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)

**Medieval and timber-frame villages**
- [S, personal project] Blockout of scale and street layout with primitives first, then modular buildings, with priority on breaking the silhouette. No structural detail on walls, jetties or roofs. — [Sourav Kumar, 80.lv](https://80.lv/articles/medieval-village-production-guide)
- [Klafke] For a castle: unique walls with modular arches, stairs and windows, still on the grid. — [Klafke](https://www.thiagoklafke.com/modularenvironments.html)
- [V, SR] Marketplace figures only: one medieval add-on lists 16 exterior walls, 8 beams, 4 gables, 10+ roof and chimney pieces, 7 floors and platforms; a half-timbered kit is described in a search summary as over 600 meshes with a standard wall of 200 x 270 x 20 cm (the listing URL was not verified against that description). — [Modular Medieval House Creator](https://superhivemarket.com/products/modular-medieval-house-creator); [Fab listing](https://www.fab.com/listings/9d3d4137-d94e-4b51-8c70-0ab3bd01dbe5)
- [SR] A 2016 stylised-village interview is summarised as using 14 interchangeable house parts for a range of sizes and shapes. — [80.lv, Stylized take on Environment Building](https://80.lv/articles/stylized-take-on-environment-building)

**Traditional East Asian buildings**
- [SR] One artist's portfolio describes a Japanese-style building kit of roof parts, wood planks and stone walls made for kitbashing many variations; a shrine roof is sold on its own as 18 meshes. No account of curved roofs, deep eaves or post-and-bay structure as kit problems was opened. — [Rick Jragon, ArtStation](https://rickjragon.artstation.com/projects/DAxNP9); [itch.io Modular Japanese Shrine Roof](https://darkcomdev.itch.io/modular-japanese-shrine-roof)

**Decayed and organic spaces (caves, tunnels, ruins)**
- [S] Pivot-and-flange for settled, crooked tunnels; shell pieces with free-placed inner walls for caves; a "floating" sub-kit of floors and walls over a grid cave kit. — [Burgess, Game Developer 2013](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [Burgess interview, 80.lv 2018](https://80.lv/articles/building-huge-open-worlds-modularity-kits-art-fatigue)
- [N] An artist modelling every stone of a ruin wall was advised that a preserved building has smoother walls, so per-stone geometry, and the corner problems it brings, may not be worth it. — [Polycount, "Creating modular walls with thickness"](https://polycount.com/discussion/232991/creating-modular-walls-with-thickness-how-to-handle-corners-and-alignment)

### Inferences
- Post-and-bay and timber-frame buildings put a visible member (post, beam) on every module boundary, which is the same device the sources use to hide seams (pillar, flange, insert). Such styles should suit kits well at wall level and badly at roof level. This is my inference; no source says it.
- Classical colonnades were not covered by any source; the nearest relevant rule is London's "repeating middle tile with end caps" for tall pillars.

### Gaps
- Classical columns and colonnades (proportion from one module, symmetry): nothing found.
- Hillside and stilt buildings: nothing found.
- East Asian curved roofs, deep eaves and bracket sets as kit pieces: nothing from a shipped practitioner was opened; only portfolio and marketplace snippets.
- Timber-frame jetties, gable framing and thatch or tile roof kits: nothing from a shipped practitioner.
