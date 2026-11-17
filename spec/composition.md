# Composition

This chapter says how a world built apart is placed in another by name: the `[import.<id>]` table,
the import's frame, how a parent names what is inside an import, what height zero is, what must
match where something leaves an import, how the two grounds join, how a parent hands a fill what it
is built from, imports inside imports, packs across imports, and what a moved import re-runs.

Words are from `vocabulary.md`. The parent is the world whose text holds `[import.<id>]`; the import
is the world it names, and the import's id is `<id>`. A world built apart compiles alone, with its
own files, namespaces, includes and pack (`documents.md` 3, 4). Rulings R8 to R11 are the source;
where a mechanism has no outside source it is marked **ours**. Every key and code here is M4; M1 to
M3 reject them with `doc.not_yet`.

## 1. Places and fills (R11)

Two ways to compose, one table:

| | A place | A fill |
|---|---|---|
| What it is | Built alone and joined at its edge: an island, a campus, a wild. | Takes its shape from the parent: a town on the parent's land, a forest, farmland. |
| What it declares | Its exits (section 7). | Its exits, and what it accepts (`[given.<name>]`, section 8). |
| What the parent writes | `[import.<id>]` and paths that answer its exits. | The same, and `give`. |
| Moving it | Re-runs nothing inside it (section 12). | Re-runs what reads a given thing inside it: its land moved under it. |

One concept, not two: a fill is an import that takes `give`. A world with `[given]` and no parent
builds from each given's `default`, so a fill is still a world built apart.

## 2. `[import.<id>]`

An import is a world table (`documents.md` 1), declared in the world file or a layer, with an id in
the world namespace (`documents.md` 3). It has the rank of the layer it is declared in.

| Key | Takes | Required | Meaning, and its absence |
|---|---|---|---|
| `file` | path | yes | The imported world file, relative to the file that declares the import (`documents.md` 7.2). |
| `at` | point | one of `at`, `on` | Where the import's frame origin, its world point (0, 0), lies in the parent's frame. |
| `on` | `{ ref, t, offset }` | one of `at`, `on` | Placed on a named place of the parent, as an instance is (`geometry.md` 2.6). |
| `face` | bearing | no | Where the import's `+y` points in the parent, degrees clockwise from north. Default 0. On `on`, added to the place's facing as for an instance. |
| `seed` | integer | no | Replaces the imported world's `seed` (`numbers.md` 7). Default: the import's own. |
| `changes` | list of names | no | Changes of the imported world applied to it, each read as `--change` reads one (`build.md` 14), relative to the import's world folder. Default: none. |
| `footprint` | name or list of points | no | What of the import is brought (6.1). Points are in the import's frame; a name is a region of the parent, a place of this import (in its frame), or a place inside another import. Default: the import's extent. |
| `blend` | formula | no | The weight, 0 to 1 per post, of the import's ground in the parent (6.3). Default `1`. |
| `authority` | tier | no | The tier of the import's ground in the parent (6.4). Default `shaped`. |
| `lift` | length | no | Metres added to the import's `surface` field when the two datums differ or are not declared (section 5). |
| `max_grade` | number | no | The seam check's limit, rise over run between neighbouring posts across the join (6.6). Default 0.5. |
| `give` | table of names | no | What the parent hands a fill (section 8). |

- `at` and `on` together are `doc.placement_twice`; neither is `doc.missing_key`.
- An import id may not be a collection's name (a top table's, or `asset`, `kind`, `material`,
  `step`, `world`): `identity.reserved` (error), since the import's diagnostics name
  the import as their collection (`a.feature.quay`) and `[import.feature]` would read back as the
  parent's own feature. Case `import_reserved`. Ours, M4 slice 4 review.
- An import takes no `scale`: a world is built at its own size (`numbers.md` 5.3).
- The import is built in its own frame, with its own extent, resolution, seed (unless `seed` is
  written), includes, kinds, pack and layers. The parent's `[let]`, kinds and fields are not
  visible inside it; only what `give` hands down is (section 8).
- A world may import one file twice under two ids; each is its own import (two hamlets from one
  file, differing by `seed` or `changes`).

```toml
# strait/world.toml: two islands built apart, and the bridge between them.
name   = "strait"
extent = [0, 0, 704, 352]
datum  = "sea"
include = ["land", "ways"]

[import.a]
file = "a/world.toml"
at   = [24, 64]

[import.b]
file = "b/world.toml"
at   = [440, 56]
face = 90

[feature.strait_bridge]
kind      = "bridge"
starts_at = "a.quay_road.end"
ends_at   = "b.landing.start"
```

## 3. The frame

The import's frame is an origin and a facing in the parent's frame (`geometry.md` 2.1, `numbers.md`
5.3): origin `at`, or the point `on` resolves to; facing `face`. A point `q` of the import's world
frame lies in the parent at `at + R(face) q`, where `R(face)` turns the import's `+y` to the bearing
`face` (clockwise from north), computed and rounded to the lattice by `numbers.md` 5.1. A facing
inside the import gains `face`, modulo 360 degrees.

- **Building alone.** Nothing the import makes depends on where it is placed: its noise is evaluated
  in its own frame with its own seed (`numbers.md` 5.3). Moving or turning a place changes no value
  inside it (section 12).
- **Instances** of the import come into the parent with their plan positions and facings taken
  through the frame. Their heights are found again on the parent's finished surface (R25,
  `surfaces.md` 4): a thing stands on the ground of the world it is in. An instance whose origin
  lies outside the footprint, or outside the parent's extent, is not brought (a point on either
  outline is inside).
- **Features** of the import come into the parent whole, taken through the frame, under scoped
  names (section 4), for the parent to name and for `features.json` (`build.md` 9.1). Their effects
  are not run again in the parent: what they did to fields arrives in the import's fields (6.2).
- **Steps** run in the frame of the world that declares them. An import's step sees the import's
  grid (`protocol.md` 2, `grid`); a parent's step sees the parent's.

## 4. Scoped names (R8)

A parent names what is inside an import through the import's id, never by coordinates:

| Written in the parent | Names | Example |
|---|---|---|
| `<import>.<place>` | A place inside the import, by the import's own place-name grammar (`vocabulary.md` 2.1) | `a.quay_road.end`, `b.landing.start`, `a.harbour_house.door` |
| `<import>.<nested>.<place>` | A place inside an import's import (section 9) | `border.college.approach.start` |
| `<import>.footprint` | The footprint as placed: a region in the parent's frame | `follows = { ref = "wild.footprint", span = [0.25, 0.5] }` |
| `<import>.<field>` in a formula | The import's finished field, read on the parent's posts (6.2) | `blend = "= smoothstep(-1, 0, a.height)"` |

- A scoped name is a named reference wherever the place-name grammar is allowed: `on.ref`,
  `starts_at`, `ends_at`, `follows`, `within`, a kind's name attributes, a formula's place argument
  (`dist('a.quay')`). Each is an edge (section 12) and is checked for direction like any named
  reference (`documents.md` 5.1), at the rank of the layer that declares the import.
- A part of a scoped name that names nothing is `identity.unknown_place`, with the nearest name
  searched in that import's namespace. A place of an import written without its prefix names
  nothing in the parent and is `identity.unknown_place` too; when the import holds that name, the
  hint is the prefixed form.
- `footprint` is reserved as the first part after an import's id: an import whose own world holds
  an entity `footprint` keeps it, reached only from inside. Ours.
- Ids, kinds and change names never collide across a parent and an import (`documents.md` 3).
  `kind:` and `tag:` selectors in the parent select the parent's own features and fields only. A
  parent that must select an import's features by kind: (open), below.
- **Produced names** from inside an import are reported as `<import>.<produced name>`
  (`identity.md` 1); hand instances as `<import>.<id>`. They appear in `instances/names.txt`, the
  change report and diagnostics, never in text.
- **Diagnostics** raised inside an import keep their own code and severity. Their subject is
  prefixed with the import's id (`a.feature.quay_road`), their `file` is the import's file as a
  path from the parent's world folder (`a/world.toml`, `../college/world.toml`) or `lib:<id>/...`,
  and their chain gains an entry `{ "through": "import", "name": "<id>" }` with the import table's
  file and line (`diagnostics.md` 1.1, 1.2). An error inside an import is an error of the build.

## 5. The datum

A world says what its height zero is with the world-file key `datum`: an id naming the reference
(`datum = "sea"`). Two worlds with the same `datum` share a zero, so their heights join with no
lift. Source: a vertical datum in surveying; the word and the rule are the core's, the name is the
author's. Absent, the world declares none: its zero is its own.

| The import's `datum` and the parent's | `lift` absent | `lift` written |
|---|---|---|
| Both written, equal | Heights taken as they are. | `import.lift_on_datum` (error): a lift would move the import off the shared zero. |
| Both written, different | `import.datum_differs` (error): say how far apart they are. | Heights lifted by `lift`. |
| Either absent | `import.no_datum` (warning), with `data.write` the line to add; heights taken as they are. | Heights lifted by `lift`. |

- `lift` is added to the import's field of role `surface` only. The core knows no other height
  (a water level is a library's tag, `fields.md` 2), which is why a shared datum is the way to join
  worlds that carry other heights. Lesson: `atlas` placed nine worlds whose zeros ran from 0 m to
  140 m and lifted each by eye.
- North is the parent frame's `+y`; an import's north turns with `face`.
- `datum` joins `extent`, `resolution`, `seed` and `chunk` in the frame statement (`graph.md` 1) and
  is written in the manifest's `frame` (`build.md` 5).

## 6. The join at the edge (R10)

### 6.1 What is brought

The import's **weight** at a parent post `p` is `w(p) = blend(p)`, raised round the places a parent
path joins (6.7, "Held"), where `p` lies inside the footprint and inside the import's extent, and 0
elsewhere. The footprint is the import's extent by
default; points written are in the import's frame and move with it; a place of the import itself
(`footprint = "a.yard"`) is in its frame too and moves with it; a region of the parent, or a place
inside another import, stays where it is when this import moves (a fill's limits, drawn by the
parent). A post on the footprint's outline is inside it, as `inside` reads an outline
(`formulas.md` 8.4). An import's footprint that reaches past the parent's extent is
`import.outside` (warning), with the area outside in `data.area`; that part is not brought.

**Kind surfaces come with their feature** (`surfaces.md` 1). Each kind surface of a brought feature
is a surface of the parent under its scoped place name, `<import>.<owner>.<surface>` (`a.span.top`;
nested, `m.n.span.top`): its bound taken through the frame point by point (every piece and hole),
its `base` read at the point in the import's frame (section 3's inverse, rounded to the lattice)
plus `lift`, as the `surface` field is lifted (section 5): a deck stays as high over the ground it
was built over. `cut`, `material` and its kind's name come unchanged. It is brought whole, as its
feature is: the footprint and the blend do not clip it (ours). From these the parent's build makes
what a world makes of its own surfaces, on its own posts and over its finished terrain:

- their meshes (`build.md` 10), `meshes/<import>.<owner>.<surface>.glb`, listed in the manifest with
  the scoped feature; a material is coloured by the parent's pack, and one it does not bind is
  `pack.unknown_material` (error) on the scoped feature, as brought paint is (6.2);
- the terrain's cut (`build.md` 6.5): a quad is cut where a brought surface with `cut` holds its
  centre, by the rule a world's own surface cuts by, read on the parent's terrain;
- the surface questions (`surfaces.md` 2): `surfaces_at` and `sample` answer them by name;
- the import's meetings (`surfaces.md` 3.3) in `meets.json`, sides scoped, moved, faces turned,
  heights plus `lift`; a side that is the import's terrain keeps its field's name and stands at the
  parent's finished surface there, and the gap is measured again between the two (ours: a reader
  of `meets.json`, a walker among them, takes a side named `height` for the parent's ground, which a
  blend ring changes; M6 close review). What must hold at each (3.4) was checked inside the import
  and is said there (section 4's prefixed subjects); the gap the join makes is in the file, not a
  diagnostic (the seam check says it, 6.6).

What stood on one inside the import stands on it here, at its height there plus `lift`: an instance
(a lamp on a deck: section 3's "found again on the parent's finished surface" is that surface), a
point feature or a place (a bookmark on a bridge, `features.json`'s `z`). The parent may **stand on** one
by its scoped name (`surface = "a.span.top"` on a point feature or an instance, `surfaces.md` 4.1), and **meets** one where its path ends at a place of the import that stands on it: the surface
its own build stood the place on, the one surface of a path whose `.start` or `.end` it is, or a kind
mount's `surface` (`ends_at = "a.span.end"`, checked by 3.4: `surface.gap` names both sides). A
parent scatter's `surface`, a parent point `on` a brought feature by the feature's name, and the
level test of `rules.md` 8.1 read the parent's own surfaces only: (open), below. Source: atlas at M6
slice 2 listed 1 mesh against city's 12, coast's 4 and monastery's 17, and its `height.cut` had 0
cells against coast's 508: coast's sea cave was solid ground and city's bookmarks on its bridges
stood in the river. The rule is ours.

### 6.2 Fields

Each field of the import is matched to the parent's field of the same name.

- The parent has a field of that name with the same `role`, `type` and `unit`: the import's
  finished field is brought (6.4).
- The roles, types or units differ: `import.field_differs` (error), naming both declarations.
- The parent has no field of that name: `import.field_unmatched` (warning), with `data.field` and the
  declaration to add in `data.write`. Nothing of it is brought. Lesson: `atlas` exported no water
  and nothing said so.
- The parent has a field the import lacks: the parent's own value stands under the import, unless
  that field is derived from one the import brings (below, "An import without the derived field").
- A field an import's generate step returns as its own (`<step>#<name>`, R32) is a produced name
  no parent can declare: it is read inside the import (by the import's rows and formulas) and is
  neither brought nor warned. To hand a step's result up, the import declares the field and the
  step returns a row on it (case `import_step`). Ours (M4 slice 4: `import.field_unmatched` asked
  for `[field.make#level]`, which no text can write).

The import's value at a parent post is read at that post's point in the import's frame, by the
rule `sample` uses (`fields.md` 1.2; bilinear between the four posts around it), plus `lift` on the
`surface` field, with one exception for a **length** (the `surface`, or a field of unit `m`): in a
**wall cell**, one whose four posts span more than twice the import's post spacing (a grade above
2, 63 degrees), the value is the nearest of the four posts (a tie to the lower index, west then
south). A field that is not a length (a tone, a density, `deg`, `ratio`, a weights plane, the
import's cover) has no wall cells and reads bilinear: twice the spacing means nothing in its unit
(case `turned_tone`; ours, M4 slice 4 review). A derived field read with its offset (below) decides
the wall cell once, by the field it is derived from, and reads both at the same post, so the
offset does not jump inside a wall cell. So a turned
quay wall stays a wall on the parent's posts, its line a staircase of whole posts, not a row of
half-height posts that draws as a saw edge. Source: the M4 sheet (`county`'s quay wall after
`move_town`, turned 8 degrees: 203 posts half-way down a wall against 0 unturned, 122 with the
rule; the rest are steeper slopes of under 2 m a cell, read bilinear). The threshold is ours; an
import placed on whole metres at quarter turns reads every post exactly and is unchanged. On a weights field, each material's weight `u[k]` is read so (0 for a material
the import does not paint), and the entry of mask `m` (6.4) mixes every material the parent holds
or the import paints, in id order, in doubles: `w[k] = (1 - m) * w[k] + m * u[k]`. This is
`fields.md` 5.2's rule with the import's weights in place of one paint: at `m = 1` the import's
paint stands whole, and weights that summed to at most 1 on both sides still do. Paint is brought
by material name, through section 10's one binding: on the parent's `material` field, a brought
material the parent's pack does not bind is `pack.unknown_material` (error) at the import, since
the build's materials are drawn from the parent's pack. Lesson: until M4 slice 4 no setts, flags
or trail paving showed in `county` or `border` (`import.weights_dropped`, retired).

**A field derived from another.** A field whose `base` is another field read whole
(`base = "= f('height')"`: a water level that is the ground wherever nothing else is written) is
derived from that field. When the parent's field and the import's of that name are both derived
from one field of that same name, the import's entry is its offset laid on the parent's ground: the
entry's value at a post is `h + (u - g)`, `h` the parent's base there (its own finished value of
the field derived from), `u` and `g` the import's values of the two fields read by the rule above;
the mix of 6.4 is unchanged. So where the import was dry the level is the parent's ground, whatever
the blend or content above the join (6.5) made of that ground, and where it held water the depth
comes with it. Any other base (a formula, `max(f('height'), sea)`) is brought by value. Source: the
M4 sheet, `border`'s walks standing in water: 2188 posts where the brought level stood more than
0.05 m over the ground (1.02 m at most), none of them wet in either import alone, 147 of them on
the walks' treads. The level was the import's ground, fixed before the join's walks cut theirs and
before the blend lowered it. The rule is ours; the alternative, each cutting kind drying its tread,
leaves the other 2041 (the blend's) and asks every kind of every library to know the water.

The parent's side of the same mix is laid the same way. The parent's own value `c` (the field as
it stands before the import's entry) was stated over the parent's ground, which the join replaced
with `h`; where `h` lies below the parent's own ground `h0`, `c` is first lowered by `h0 - h`, so the
parent's water keeps its depth over its own ground rather than its level over ground that is no
longer there: the import's entry sets the post to
`(1 - m) * (c - max(0, h0 - h)) + m * (h + u - g)`, which is `h + (1 - m) * (c - max(h, h0)) +
m * (u - g)`: a depth mixed from the two sides' depths, so the joined post is never wetter than
either side alone. `h0` is the parent's head view of the field derived from (`authority.md` 2.3:
its base and rows, no feature's effect and no import's entry): the ground the parent's rows, and a
generate step reading the head view, stated their water over; so each chunk of the derived field
that brings an import reads that head view's chunk of the same window (`graph.md` 2: a row under a
parent's effect changes the head view and not the build view, case `derived_lower_head`). Where `h` is at or above `h0` the
entry is the mix above, unchanged: the parent's level is never raised by the import's ground. The
import's anchored entry (6.4) comes after its first and mixes as above with no lowering (the first
already laid the parent's value on `h`). Source: the M6 slice 2 on-foot review of `atlas`: 137,886
posts of the blend rings stood wet and 15 of 126 bookmarks in water, where the places' lifted
ground lies below the plain and the plain's water (its terrain step's level, one metre under the
plain's own ground where dry) was mixed in by `1 - m`. Ours.

**An import without the derived field.** When the parent's field is derived from a field the
import brings (its `surface`, say) and the import declares no field of the derived field's name, the
import is dry under it: it brings an entry as above with `u - g = 0` (its first entry only; it has
no anchored cover of the field). The parent's water was stated over the parent's ground, which the
import replaced; the import said nothing of water over its own. Source: `college` in `atlas`
declares no water field, and the plain's standing water (the terrain step's row, in a hollow of the
plain's ground) stood up to 7.2 m deep over its lowered lawns; a row of the parent cannot reach
under a step's row to dry it. Ours; an import that wants the parent's water over it declares the
field (`base = "= f('height')"`) and brings it.

A parent whose post spacing is coarser than the import's loses detail;
`import.coarser` (info) says so once per import with `data.ratio`. No filter is applied: ours, (open)
below. The same values are what a parent formula reads as `<import>.<field>`; outside the footprint
it reads 0, and `inside('<import>.footprint')` says where it is defined.

### 6.3 `blend`

`blend` is a per-post formula, range 0 to 1 (`formulas.md` 6.1). It reads `x`, `y` (the parent's
frame), `d` (the distance from the footprint's outline inward, metres, 0 on the outline), the
parent's `let` names, any parent field the import does not bring, any `<import>.<field>`, and the
functions of `formulas.md` on the import's places by their scoped names (`dist('a.south_gate', 40)`,
the place in the parent's frame). A name of three parts or more (a path's end, `a.track.start`,
or a place of a nested import) is not read yet: `doc.not_yet`, whose hint names the hold of 6.7,
which holds such a place without a formula, and a parent point `on` it as the form that works
(owed; M6 close fresh-agent run `weir`, which wrote 6.7's form with `glade.track.start`). A
parent field the import brings, read by its own name in `blend` (`f('height')`, as any field is
read), reads itself through the blend: `graph.cycle`. Out of range is `formula.range`.

- Default `1`: the import's ground holds to its footprint's edge, and a step there is reported
  (6.6). Not the spike's falloff over a fixed apron, which bent the ground on both sides by a number
  nobody chose.
- A blend that reads the ground: `blend = "= smoothstep(-1, 0, a.height)"` brings an island's land
  and leaves the parent's sea bed where the island is under water (R8, owed since `strait`).

### 6.4 Which side yields: authority

Each brought field is two entries in the parent's field, ordered with every other entry by R21 and
R29 (`authority.md` 4). Source: R10 and the TODO "decide which side yields by authority, not by key";
the split into two entries is ours.

| Entry | Value | Mask | Tier |
|---|---|---|---|
| the import's ground | the import's value (6.2) | `w` | the import's `authority` (default `shaped`) |
| the import's anchored ground | the same | `w` times the import's **anchored cover**: at each import post, the largest mask of any `anchored` entry of the import on that field | `anchored`; omitted when the import's `authority` is `anchored` |

- So a parent's `anchored` feature over a `shaped` import is applied after it and holds (a bridge's
  ramp on an island); a parent's `shaped` or `derived` feature over it is applied before it and
  gives way; and what the import anchored at home (a quay, a terrace) still holds in the parent
  against any parent statement below `anchored`.
- A parent `anchored` feature meeting the import's anchored ground is two firm statements:
  `authority.conflict` or `authority.squeezed` by `authority.md` 5, with the import's anchored
  feature of largest mask at the reported post in `related` (its import file and line). Overruling
  ground an import anchored is never silent (R10). This settles `authority.md`'s open line: an
  error where both masks are 1, a warning where they fade.
- When the import's anchored ground is the later entry (the import's layer ranked above the parent
  feature), the subject is still the parent feature: it leaves its value and the import asks. An
  import's ground reads nothing of the parent, so the hint offers making the parent feature
  `shaped`, moving it off the import, or ranking the import's layer below it (case
  `anchored_clash_above`).
- **Where the two entries stand.** Each is one entry of the layer that declares the import, at
  that layer's rank, nested in nothing and with no `order` (0); its id for R29's last tie-break is
  `<import>` (`a`), bytewise, against the parent's own ids. So ties inside one tier are R29's:
  nesting, `order`, layer rank, then id bytewise. A parent `shaped` entry in the same layer as a
  `shaped` import is ordered by its id against the import's, which nobody chose: put parent content
  over an import in a layer ranked above (6.5) or give it `order`.
- **The master above its parts by default** (TODO "Composition") is declined: R29 has one order
  and no default by origin, and the layer above (6.5) says the same thing by rank. Ours.

### 6.5 Content over the join: a layer above

R10's join content (a ridge along the edge, the walks through it, the growth across it) is
parent content in its own layer with its own owner, **ranked above the layer that declares the
import**. No other key is needed:

- its entries come after the import's in the same tier (R29's layer rank), so a `shaped` ridge on a
  `shaped` import is not blended away (the spike's `border`: 7 m kept as 4.1 m without it);
- a generate step declared in it reads the joined ground in its stratum (`authority.md` 4.4), and
  `<import>.<field>` for each side's own: this is **a step at a join**, code handed both sides
  (TODO "a step that runs at a join"); a step in a layer at or below the import's rank that asks for
  `<import>.<field>` is `graph.read_outside`, naming the import's layer. A step's full view of a
  brought field keeps an import's entries only when the import's layer is at or below its own
  (`authority.md` 4.4: entries of a higher layer are left out); its head view holds no import
  entry, as it holds no effect (6.4: an import's ground enters as tier entries, not rows). Only a
  generate step reads `<import>.<field>`; to a lint or plain step it names no field (TODO: a named
  refusal for it).

The spike's `after = "parts"` and the vocabulary's `after = "imports"` are dropped (principle 10):
rank already says it. `after` on a layer is `doc.unknown_key` with the fixed hint "`after` was
dropped: rank the layer above the layer that declares the import (`composition.md` 6.5)".

### 6.6 The seam check

After the build, for the parent's `surface` field, every pair of neighbouring posts (east or north)
whose two weights differ is a **seam pair**. Its grade is the difference of their finished heights
over the post spacing. A pair whose grade is above the import's `max_grade` is a step at the join,
unless a parent `anchored` entry on the field has mask 1 at either post: then a named thing owns
that ground (R18), such as a way that crosses the edge at an exit. A `shaped` entry excuses
nothing: a parent road of a `shaped` kind across the edge still has its pairs checked, so either
hold its ground with an `anchored` entry or make the two grounds meet at the exit (6.7). Pairs on the parent's extent edge
have no neighbour outside and are never seam pairs, so a footprint side along the extent needs no
raised limit (TODO "`max_step` per side").

One `import.seam` (warning) per import: `data.grade` the worst pair's grade, `count` the steep
pairs, and its position the post of the worst pair that lies inside the footprint. Default 0.5: ours, a slope steeper than one in
two across a line nobody drew; (open) below.

### 6.7 Places the join alters

A place of the import that a parent named reference names (an exit it answers, a path's end, an
`on.ref`) and whose weight is below 1 stands on ground the import did not make (after the hold
below, only a place outside the footprint).
`import.place_blended` (warning) names it, with `data.weight` and `data.moved`, the height
difference between the parent's finished surface and the import's own there (R8: "a named place
that the blend would alter is reported").

**Held.** An exit on the import's edge under a fading blend would be such a place (its weight is
the blend's at `d` near 0), so the import's ground is held round every place a parent path joins:
each place of the import that a parent feature's `starts_at` or `ends_at` names, or a parent
feature or instance's `on.ref` names, that is a point (an exit, a point feature, a path's `start`
or `end`) inside the footprint. At each post of the footprint the weight is
`max(blend, 1 - smoothstep(12, 30, r))`, `r` the distance in metres to the nearest such place: the
import's ground whole within 12 m of it, faded to the blend's by 30 m. The parent's way that
answers the place then meets the import's ground at the edge, and a step there is the seam check's
(6.6), on the parent's side, where the parent's way is the parent's to grade. A place outside the
footprint is not held (nothing of the import is brought there) and is still
`import.place_blended`, with `data.weight` 0. Sources: the M4 slice 4 fresh-agent run (`hamlet`)
found the hold by trial, written in its `blend` as
`max(smoothstep(0, 40, d), 1 - smoothstep(12, 30, dist('a.south_gate', 40)))`; in `atlas`, after
13 blends written without it, three ways arrived on a cliff: `vale.valley_road.start` stood at
14.0 m and the vale's road fell to -22.9 m within 25 m inside the vale's ring (`place_blended`
moved -15.4 m at the vale, -23.9 at the relief, -11.2 at the market). The hold, its radii (from
`hamlet`) and that it is not a key are ours: every author joining a way met it, and a key to turn it
off has no case.

### 6.8 Two imports

Two imports whose footprints, as placed, intersect with an area above 0 overlap: `import.overlap`
(warning) once per pair, both ids, and `data.area` the area of the intersection in square metres,
computed on the lattice (`geometry.md` 3). Their entries take R29's order; the overlap says nobody
chose it. `import.outside` and `import.give_outside` measure their `data.area` the same way.

## 7. Exits (R9)

A kind marks its point features as exits with `exit` (`authority.md` 1): `exit = true`, or
`exit = ["width", "class"]`, a list of the kind's attributes an answer must match. The core owns
the flag and the word; the library owns the kind's name and the attributes.

- **Inside its own world** an exit is a point feature like any other. Built alone, nothing answers
  it and nothing is reported.
- **Where it must lie.** Inside the import's footprint, edges included. Outside it is
  `import.exit_outside` (error): it would leave from ground that is not brought.
- **Answered.** In a parent, every exit of every direct import (section 9) must be named by at least
  one named reference of the parent (`documents.md` 5.1): a path's `starts_at` or `ends_at`, an
  `on.ref`, or a name attribute. What answers is the parent's choice: a way that goes on, a way to
  another import's exit, or a deliberate end (a library kind placed `on` it). The core asks only that
  the parent said something.
- **Forgotten.** An exit no parent reference names is `import.exit_unanswered` (error), subject
  `<import>.feature.<exit>`, at the import table's span, with the exit's file and line in
  `related`. `data.write` holds the line that answers it: `ends_at = "<import>.<exit>"`. Its hint:
  "end a path there with `ends_at = "<import>.<exit>"`, or place a feature `on` it that ends it".
- **Matched.** For each attribute in the exit's `exit` list, each answering feature's attribute of
  that name (after defaults and formulas, `authority.md` 1.1) must equal the exit's: strings and
  bools exactly, numbers by value. A difference, or an answer whose kind does not declare it, is
  `import.exit_mismatch` (error), naming the attribute in `data.attribute`, both values in
  `data.exit_value` and `data.answer_value`, both spans, and the line that fixes it in `data.write`
  (`width = 13`). A feature `on` the exit that ends it compares too: a deliberate end of the wrong
  width is still wrong. Challenge 55 recorded `width`; which attributes is the library's word, the
  comparison is the core's.

Sources: R9; the TODO calls the contract "the same as sockets on a modular kit". Making the list
the value of `exit` rather than a second key is ours (principle 10).

## 8. Give and given (R11)

A fill declares what it accepts; the parent hands it down.

```toml
# town/world.toml: a fill. Alone, it builds on flat land with one district.
[given.land]
accepts = "field"
default = "= 0"

[given.limits]
accepts = "region"                  # alone: the whole extent

[given.districts]
accepts = "features"
kind    = "zone"                    # the town's own kind; given features take it
default = "kind:zone"               # alone: its own zones

[given.density]
accepts = "number"
default = 0.6

[field.height]
role = "surface"
base = "= land"
```

```toml
# county/world.toml: the parent.
[import.town]
file = "town/world.toml"
at   = [400, 300]
give = { land = "height", limits = "town_limits", districts = "kind:district", density = 0.8 }
```

| `accepts` | `default` | The parent gives | Read inside the fill as |
|---|---|---|---|
| `region` | a region of the fill; absent: the fill's extent | a region feature, or a place inside an import | the region named `<name>` (`inside('limits')`, `within = "limits"`) |
| `field` | a formula, required | a field name | a field named `<name>`, read in formulas; no role |
| `features` | a `kind:` selector over the fill's own features; absent: none | a `kind:` selector, or a list of names | the selector `given:<name>`; each feature named `<name>.<parent id>` (below) |
| `number` | a number, required | a number | a name formulas read, as a `let` name |

- `[given.<name>]` is a world table; `<name>` is in the fill's world namespace, so a field or
  feature of the same id is `identity.duplicate_id`.
- **Frames.** A given region or feature is taken into the fill's frame by the inverse of the
  import's frame (section 3). A given field is read on the fill's posts by `sample`'s rule from the
  parent's posts, as the parent's field stands without this import's own entries (6.4): a fill on
  the parent's land does not read itself through the land it brings back (case `give_fill`;
  correction of slice 2, the text said nothing of which value is handed and every reading but
  this one is a loop); a fill whose extent, as placed, reaches past the parent's is
  `import.give_outside` (error), with `data.area`. A number is handed as written.
- **A given field is a field of the fill's build**: written to `fields/<name>.npy` like any scalar
  field, its default when built alone and the parent's values when given, so a loader or step of
  the fill sees the ground it was handed. A world that turns its base into a given (`quay`'s
  `land`) so gains one output file and keeps its other fields' values (within 2e-6 m on `quay`:
  the formula is evaluated as a field, then read). Ours; the text said nothing (M4 slice 3 review).
- **Given features** take the given's `kind` (the fill's own kind), the parent's authority, and
  the attributes written on them, by name; the parent kind's defaults and formulas stay the
  parent's and are not handed down. A written attribute the fill's kind does not declare is
  dropped with `import.given_dropped` (warning), naming it. They belong to the fill's lowest rank,
  below every layer of the fill, so every layer may read them. Ours.
- **Their names.** A given feature is named `<name>.<parent id>`, the parent id whole, dots
  included (`districts.d1`, `districts.a.x` for a parent's scoped `a.x`). The place-name grammar
  reads a first part that names a given of `accepts = "features"` so: the longest run of the
  following parts that is the id of a feature given under it is the parent id, and what follows
  is a part (`districts.d1.start`). A given's name is in the fill's world namespace, so no feature
  of the fill shares that first part. Ours.
- **Defaults stay.** A fill's default features are the fill's own text and stay in its build when a
  parent gives; `given:<name>` resolves to what was given. A fill that must not keep them puts them
  in a layer and the parent requests a change that removes it (`changes`).
- `give` naming a name the fill does not declare is `import.given_unknown` (error), with the nearest
  declared name. A given of another form than `accepts` (a field handed to a region) is
  `import.given_type` (error), naming both. An `accepts` with no required `default` is
  `doc.missing_key`. A given the parent does not give takes its default, silently: opt-in per input
  (R11).
- **What a give may read.** Any parent node, and any node of an import, the same import included,
  that does not reach the given node (`graph.md` 9): a parent may draw a fill's limits along the
  fill's own arterials. One that does is `graph.cycle`. This answers "let the parent see the part
  before it gives": the part is built up to what reads the given, no further.
- `given` and `[let]` stay two tables: a `let` is the world's own constant, a given is what a
  parent may hand in. A given number is a `let` the parent may set (TODO "decide whether `given`
  and `params` are one table").

## 9. Imports inside imports

Followed. An import's own imports are built as part of it, as they are when it builds alone, and
come into the parent through it: the parent names `a.b.<place>` (section 4). Reason: a world that
builds alone must build the same inside another (`border` imports `college`; `atlas` imports
`border`), and rejecting would make a world unusable as an import because of how it was built.

- The parent's obligations are its direct imports' exits only. A nested import's exits are its
  middle world's to answer, and an unanswered one is the middle world's `import.exit_unanswered`,
  raised in the parent's build with its chain through both imports. A middle world that passes an
  exit on declares its own exit `on` the nested one: that answers one and offers the other.
- `give` reaches direct imports only; a middle world hands on what it was given with its own `give`.
- Worlds that import each other, at any depth, are `import.cycle` (error), naming the files of the
  loop in order. One file imported at two places in the tree is two imports, not a cycle.

### 9.1 Networks and cells across imports (R13)

- **An import's own** networks and cells are built in its frame, so their produced names (faces,
  cells, `identity.md` 2.3) are made from digests in that frame: moving or turning the import
  changes none of them. They are reported as `<import>.<produced name>` (section 4) and keep that
  name when the import moves.
- **A parent `[network]` or `[cells]`** may list scoped paths and regions among its sources
  (`sources = ["main", "a.quay_road"]`), read as placed through `placed:<id>/<name>` (section 12).
  Its produced names are the parent's, made from the sources as placed, so moving the import is a
  move of that source: names follow R13's rules for a moved source (`identity.md` 2.3), as for a
  moved parent path.
- **Naming an import's face or cell from the parent** (`a.<network>.<face>`): (open), below.

## 10. Packs and asset ids

Each world resolves asset ids with its own `pack` (`assets.md` 1.2), imports included: a world
built alone looks the same placed. In a build, an asset id or material name is one binding:

- Two worlds of one build that bind one asset id or material name differently (an asset by a
  different statement or file digest, a material by its `colour`) is `import.binding_differs`
  (error), naming the id,
  both packs and the worlds that use them. Hint: bind it the same, or rename it in one pack. One
  binding in two packs, or one pack shared, is one asset (`documents.md` 9).
- Asset facts are measured once per asset in the build (`assets.md` 4); `asset.revised` lists the
  import's instances under their scoped names.
- **The build's assets are every world's** (ours, M6 slice 2): an asset an import's pack binds and
  the parent's does not is brought into the build as that one binding. The manifest lists it with
  `pack`, the import's scoped id, and its files under that pack's root (`build.md` 4, 7); a
  chunk's budget counts it (`build.md` 5.1); `pack.query` and a plain step's `pack` hold it
  (`protocol.md` 2.2) beside the parent's own, with `"*"` or its id, not under an `@<taxon>`. The
  parent's own text still resolves ids with its own pack alone: an instance or scatter of the
  parent naming it is `pack.unknown_asset`, and a taxon selects only the parent's assets, so an
  import never changes what the parent places. Why: a reader, a budget and a map step read the
  build, and an asset missing there drew nothing and counted 0 triangles (atlas, M6 slice 2).
- Swapping the parent's pack swaps the look of what the parent places, not an import's: each
  import's pack is the import's text.

## 11. Changes across imports

- `changes` on an import applies the import's own changes to it, by name or path as `--change`
  reads one, relative to the import's world folder. An unknown one is `doc.unknown_change`, chain
  through the import.
- A parent's change may write the parent's `[import.<id>]` keys (move it, turn it, request other
  changes, give other things), never the import's own tables (`documents.md` 9). A difference
  inside an import is one of the import's changes. There is no way to write one: an import's
  tables are named only by scoped names, which are not ids, so a change table such as
  `[feature."a.quay"]` is `identity.bad_id`, and a table of the bare id (`[feature.quay]`) is the
  parent's own entity of that id. No new code.
- The change report lists the import's things under scoped names, with causes in the parent's
  terms (`import.b.at`) where the import moved, and in the import's (`b.feature.landing.path`)
  where it changed inside.

## 12. The graph

An import is a graph of its own, its node ids prefixed `<id>.` (`graph.md` 9). The prefix is in no
key: one world imported twice, or built alone, shares its results in one store.

**Statements.** The import table makes four kinds of statement, as the world file makes two
(`graph.md` 1): `import.<id>.frame` (`at`, `on`, `face`, `footprint`), `import.<id>.join`
(`blend`, `lift`, `authority`, `max_grade`), `import.<id>.build` (`seed`, `changes`), and one
`import.<id>.give.<name>` per name of `give`. `file` enters no node's key: it is a source of the
build key (`graph.md` 6.2), and the import's nodes are keyed on the import's own statements, whose
digests hold no path. So two parents that reach one world by different paths share its nodes.

**Families.**

| Family | Node id | Made from | Value | Kept in the store |
|---|---|---|---|---|
| `import` | `import:<id>` | `import.<id>.build`; the digests of the import's own statements (its frame, `world.pack`, every table, its imports' in turn) | the import's build key in its own frame | digest only |
| `placed` | `placed:<id>` | `import.<id>.frame`; the parent's frame; the place `on` names; the region `footprint` names, or the import's node of the place it names | the frame (origin, facing) and the footprint as placed, with `import.outside` and `import.give_outside` | digest only |
| `placed` | `placed:<id>/<name>` | `placed:<id>`; the import's one node that makes `<name>` (its shape, instance, footprint or made node, `graph.md` 1) | that place in the parent's frame | digest only |
| `brought` | `brought:<id>/<field>/w:<window>` | per parent chunk the footprint meets: `placed:<id>`; `import.<id>.join`; the import's finished chunks of `<field>` over the window the chunk maps to, grown by one post (the bilinear read, 6.2); the import's mask nodes of its `anchored` entries on `<field>` over that window (the anchored cover, 6.4) | the import's value, its weight and its anchored cover at the chunk's posts | yes |
| `given` | `<id>.given:<name>` | `import.<id>.give.<name>`; the parent's node the give names; `placed:<id>` | the given thing in the import's frame | digest only |
| `joincheck` | `joincheck:<id>/seam/w:<window>` | per parent chunk the footprint's outline meets: `brought:<id>/<surface>` and the parent's finished surface chunks there, grown by one post; `import.<id>.join` (`max_grade`); the parent's `anchored` entries' masks there | that chunk's seam pairs; their `import.seam` is gathered per import | digest only |
| `joincheck` | `joincheck:<id>/exits` | `placed:<id>/<exit>` for each exit of the import; the merge (every parent named reference, `graph.md` 1); the attribute nodes of each answering feature | `import.exit_unanswered`, `import.exit_mismatch`, `import.exit_outside` | digest only |
| `joincheck` | `joincheck:<id>/blended` | `placed:<id>/<name>` of each place a parent reference names; the `brought` and parent surface chunks at those places | `import.place_blended` | digest only |
| `joincheck` | `joincheck:<id>/overlap:<other>` | `placed:<id>` and `placed:<other>`, for each pair `<id>` < `<other>` bytewise | `import.overlap` | digest only |

**An import's steps.** An import's generate steps run in the parent's build, in its store, before
the import is built, rank by rank as in a build alone (`protocol.md` 5): the import's `step` nodes
are the parent's `<id>.step:<name>` and `<id>.step:<name>/i:<item>`, and the run report lists them
so. Their keys are the import's own: its statements, its seed and what it is given (the memo of a
worker's reads holds the given's digest, `graph.md` 5.1), never the frame. So a moved place finds
every call in the store, a changed given re-runs only the calls whose reads changed, and a world
built alone or imported shares its step results. A fill's first build, which only places it before
the parent gives, runs no step. `--rerun-step` names an import's step by its prefixed id
(`--rerun-step a.make`, `m.n.make` nested); a bare id names only the parent's own step (ours, M4
slice 4 review: one set handed to every import re-ran `a.step:make` under `--rerun-step make`). An import's lint steps do not run in the parent (the parent's own
lints read the joined build). Correction of slice 4: the import's other nodes (fields, shapes,
places) are not yet the parent's: `import:<id>` builds them in the process, once per (file,
seed, changes, given), so an unchanged parent rebuild still builds every import (owed, TODO).

**Given features, per item.** A fill's step may run once per feature a parent gave it:
`for_each = "given:<name>"`, the item named `<name>.<parent id>` (section 8). Inside a call,
`features("given:<name>", ...)` selects the features given under `<name>` (`protocol.md` 4.3);
when none was given, the given's `default` selector stands in. Ours.

**Edges.** A scoped name `<import>.<name>` reads `placed:<id>/<name>`, never the whole import;
`<import>.footprint` reads `placed:<id>`; a parent field chunk reads `brought:<id>/<field>` for that
chunk; a formula's `<import>.<field>` reads the same node; a fill's node that reads a given reads
`<id>.given:<name>`; the `give` edge runs from the given node to the parent's node it names.
`import:<id>` is read by the write nodes of `features.json` and the instance columns and by no node
a give may read.

So a give reaches only what it names: a parent region drawn from `town.arterial` reads
`placed:town/arterial`, which reads the fill's shape of `arterial` and what that reads. It is a
`graph.cycle` only if that reaches `town.given:limits` (section 8, case `give_reads_fill`), and a
parent node that reads one name of an import does not run again when another name changes.

What a change re-runs, which conformance checks by ran sets:

- **A place moved or turned** (`at`, `face`): no `<id>.` node runs; `import:<id>` is reused.
  `placed:<id>` and each `placed:<id>/<name>` run, and `brought` and the seam check run for each
  parent chunk its old or new footprint meets, with the parent's field chunks there and every
  parent node that names one of its places (a bridge to its quay). Parent chunks the footprint
  never met are reused.
- **A fill moved**: its given things moved in its frame, so the fill's nodes that read a given run
  again, as their land changed. Nodes of the fill that read no given are reused.
- **A given thing changed** (a square moved): only the fill's nodes whose read of the given changed
  run. With a step `for_each = "given:districts"` that reads `given:squares` within its item
  (`features("given:squares", within = <item>)`, `protocol.md` 4.3), the call of the district that
  holds the square, before or after the move, runs; every other district's call is reused by its
  trace (`graph.md` 5.1, 5.3). This is the plan's "a moved square re-runs only its district".
- **A change inside the import** (one of its `changes`): its own nodes run as in a build alone;
  the parent re-runs only what reads a node whose value changed (early cutoff, `graph.md` 3.1).

## 13. Decisions

Each (open) item this chapter was asked to settle, with its reason. **Ours** marks a mechanism with
no outside source.

| Item | Decision | Reason |
|---|---|---|
| Placement | `at` or `on`, `face`; origin is the import's (0, 0); no `scale` | One grammar with instances (`vocabulary.md` 6). |
| Import placed `on` (`numbers.md` Open) | Allowed, resolved as an instance's `on` | An interior hung on a mount is the M5 case; no new rule. |
| A parent setting an import's seed (`numbers.md` Open) | Allowed: `seed` on the import | Two places from one file must differ without editing it. |
| Resampling (`fields.md` Open) | `sample`'s bilinear rule at the post's point in the import frame; no filter; `import.coarser` says when detail is lost | One reading rule for every field; a filter waits for a world that shows the loss. Ours. |
| Datum (`geometry.md` Open, moved here from M6) | `datum = "<id>"` on the world; equal names join with no lift; else `lift`, else a warning | `atlas` lifted nine worlds by eye; a name is the least that makes zero declared. Ours. |
| Lift | On the `surface` field only | The core knows no other height. |
| Which fields join | By name, with role, type and unit equal | Fields are named by the author; the role check catches a mismatch. |
| Blend | Default `1`; a formula that may read `d` and `<import>.<field>` | No hidden apron; a blend that reads the ground is owed by R8. |
| Which side yields | Authority, with two entries per field: the import's tier, and its anchored cover as `anchored` | R10 and the TODO; the import's firm decisions stay firm in the parent. Ours (the split). |
| Overruling an import's anchored ground (`authority.md` Open) | `authority.conflict` / `authority.squeezed` as inside one world | Principle 3: a clash between two firm decisions is an error. |
| A layer after imports | Rank above the import's layer; `after` dropped | Rank already orders entries and decides what a step reads (R21, R29); principle 10. Amends R10's wording. |
| A step at a join | A generate step in a layer ranked above, reading the joined field and `<import>.<field>` | No new mechanism. |
| Seam check | Grade between neighbouring posts of differing weight above `max_grade` (0.5), excused under anchored ground and at the extent edge | The spike's `max_step` per part had to be raised for sides on the world edge. Ours. |
| Exit attributes compared (`documents.md` Open) | Those listed in `exit = [...]`; exact equality | The library knows which matter; the core compares. |
| What answers an exit | Any parent named reference to it | The core asks that something was said, not what (R9). |
| Exits on the boundary | Inside the footprint, not necessarily on its edge | A river's mouth may stand inside; what matters is that it is brought. |
| Scoped names | `<import>.<place>`, `<import>.<nested>.<place>`, `<import>.footprint`, `<import>.<field>` | R8; one grammar. |
| Given forms | `region`, `field`, `features`, `number`; given features named `<name>.<parent id>` | R11; numbers settle the `params` question. Naming ours. |
| Nested imports (`documents.md` Open) | Followed; obligations and `give` reach direct imports only | A world must build the same placed as alone. |
| Packs (`documents.md`, `assets.md` Open) | Each world's own pack; one binding per id in a build, else an error | A world looks the same placed; no silent winner. |
| Change files requested by an import (`documents.md` Open) | `changes`, read as `--change`, from the import's folder | One rule for naming a change. |
| Trust for an import's steps (Open, M4 slice 4) | Trusted as the parent's are: `via trust` of the parent lists every import's generate steps (its lints never run in the parent, 12), at any depth, as `<id>.<step>`, and trusts them by the same digest | The parent names the import's file in its own text, so its code is the parent's choice as a `--lib` library's is; a second trust list would only be asked and agreed again. Ours. |
| Graph | Families `import`, `placed` (the frame, and one node per scoped name), `brought`, `given`, `joincheck`; placement keys outside the import's keys; `file` in no key | A moved place re-runs only what reads it; a give reaches only the names it reads. |

## Diagnostics

Ids, places, layers and formulas use their own chapters' codes (`identity.unknown_place`,
`doc.placement_twice`, `doc.unknown_change`, `formula.range`, `graph.cycle`, `graph.read_outside`,
`authority.conflict`, `num.seed_type`).

| Code | Severity | Says |
|---|---|---|
| `import.cycle` | error | Worlds that import each other; the files of the loop, in order. (M4) |
| `import.exit_unanswered` | error | An exit of a direct import that no parent reference names; the line that answers it in `data.write`. (M4) |
| `import.exit_mismatch` | error | An answer to an exit whose attribute differs from the exit's; `data.attribute`, `data.exit_value`, `data.answer_value`, the fixing line in `data.write`. (M4) |
| `import.exit_outside` | error | An exit lies outside its import's footprint. (M4) |
| `import.datum_differs` | error | The import and the parent declare different datums and the import has no `lift`; `data.datum`, `data.parent_datum`. (M4) |
| `import.lift_on_datum` | error | `lift` on an import that declares the parent's datum. (M4) |
| `import.no_datum` | warning | The import or the parent declares no datum and the import has no `lift`; the line to add in `data.write`. (M4) |
| `import.field_differs` | error | An import's field and the parent's field of one name differ in role, type or unit; `data.field`. (M4) |
| `import.field_unmatched` | warning | An import's field the parent does not declare, so not brought; `data.field`, the declaration in `data.write`. (M4) |
| `import.coarser` | info | The parent's posts are coarser than the import's, so detail is lost; `data.ratio`. (M4) |
| `import.outside` | warning | Part of an import's footprint lies past the parent's extent and is not brought; `data.area`. (M4) |
| `import.overlap` | warning | Two imports' footprints, as placed, intersect with an area above 0; `data.area` (6.8). (M4) |
| `import.seam` | warning | A step in the ground across the join steeper than `max_grade`, worst first by data.grade. (M4) |
| `import.place_blended` | warning | A place the parent names stands where the import's weight is below 1; `data.weight`, `data.moved`. (M4) |
| `import.binding_differs` | error | One asset id or material name bound differently by two packs of the build; `data.id`. (M4) |
| `import.given_unknown` | error | `give` names a name the import does not declare in `[given]`; the nearest name. (M4) |
| `import.given_type` | error | What is given is not the form the given `accepts`; `data.accepts`. (M4) |
| `import.give_outside` | error | A given field does not cover the fill's extent as placed; `data.area`. (M4) |
| `import.given_dropped` | warning | A given feature's attribute the fill's kind does not declare was dropped; `data.attribute`. (M4) |

## Conformance

Cases are stored and run as `conformance.md` says: one folder per case under
`spec/cases/composition/`, with a `case.toml`. Every case is milestone 4 but where its row says M6. Small worlds of 8 m to 32
m at 1 m posts; heights are chosen so that every checked post is an exact float. The runner's
re-runs (`conformance.md` 5) apply to every build.

| Case | M | Input | Action | Must hold |
|---|---|---|---|---|
| moved | M4 | Two places `a`, `b` and a parent path from `a.quay.end` to `b.landing.start`; a change moving `b` 4 m east | build; build with the change | The path's ends at the placed points, (8, 8) to (40, 8), then to (44, 8); in the second build `placed:b` ran, `import:a` and `import:b` were reused, and neither import's shape nodes ran. |
| turned | M4 | `a` holding instance `hut` at (2, 2) and a point `spot` at (4, 0), placed at (20, 20) with `face = 90` | build | `a.hut` at (22, 18), facing 90 degrees; a path `ends_at = "a.spot"` ends at (20, 16); a parent instance `on = { ref = "a.spot" }` (unfaced) at (20, 16) facing 90 degrees. |
| own_seed | M4 | `a` with a noise height, placed at (4, 4), then at (6, 4); `a` built alone | build each | Each placed window of the parent's height equals `a`'s own height bit for bit: moving changed no noise. |
| seed_set | M4 | The same import with `seed = 7`; then `seed = "x"` | build each | The parent's height differs from the build without `seed`; then `num.seed_type`. |
| placed_on | M4 | An import `on = { ref = "spot" }`, `face = 180`, the point `spot` at (10, 10), `a`'s field `tone = x`; then `spot` moved to (14, 10) | build; build again on one store | `a.hut` (at (2, 2) in `a`) at (8, 8), parent `tone` 2 at post (8, 8); then `a.hut` at (12, 8) and `tone` 6 there: `placed:a` reads the place `on` names. |
| import_file_edited | M4 | `a`'s `height` from `h.npy` (2), placed at (8, 0); then the file rewritten to 5 | build; build again on one store | The parent's height 2 at post (12, 4); then 5: `import:a` holds the import's input leaves. |
| blend_reads | M4 | `blend = "= f('mix')"` with parent `mix` 1 and `a`'s `tone` 8; then `mix` 0.5 | build; build again on one store | Parent `tone` 8 at post (12, 4); then 4: a brought chunk reads what its blend reads. |
| seam_file_edited | M4 | The parent's `height` from `p.npy` (2), a flat import at (8, 0); then the file rewritten to 5 | build; build again on one store | `import.seam` with `data.grade` 2; then 5: the join's checks read the finished surface and inputs. |
| import_step | M4 | `a` declares `tone` (0); its own generate step returns a field `level` of 3 and a row on `tone` reading it; then `a` moved 4 m east | trust the parent; build; build again on one store | Parent `tone` 3 at post (12, 4), 0 at (2, 4); no `import.field_unmatched` for the step's own `make#level`; then `a` moved to (12, 0): `a.step:make` and `import:a` not run, `placed:a` ran, `tone` 3 at (16, 4) and 0 at (10, 4). |
| scoped_names | M4 | A parent and an import each holding instance `hut`; a parent bookmark `on.ref = "a.hutt"`; then `on.ref = "hut2"` with `hut2` only in the import | merge each | No `identity.duplicate_id`; both `hut` and `a.hut` built; then `identity.unknown_place` nearest `a.hut`; then `identity.unknown_place` hinting `a.hut2`. |
| weights_brought | M4 | Parent and `a` (8 m square at (8, 0)) each with a `weights` field `mix`; the parent paints `sand` everywhere, `a` paints `moss`; then `blend = 0.25` | build each | No diagnostic; post (12, 4) `moss` 255 and `sand` 0, post (2, 4) `sand` 255 and `moss` 0; then at (12, 4) `moss` 64 and `sand` 191 (`build.md` 6.2's bytes of 0.25 and 0.75). |
| derived_brought | M4 | Parent height 0 and `a`'s 4 at (8, 0), `blend = 0.5`; both declare `level` with `base = "= f('height')"`; `a` raises its level 1 m over its west half | build | Height 2 and `level` 2 at post (12, 4) (dry: not 3); `level` 2.5 at (9, 4) (not 3.5); 0 at (2, 4). |
| derived_lower | M6 | Parent height 4, `a`'s 0 at (8, 0), `blend = 0.5`; both declare `level` with `base = "= f('height')"`; the parent's `level` has rows `max(v, 3)` (under its ground: dry) and `max(v, 5)` west of x = 10 (1 m deep) | build | Height 2 at (12, 4); `level` 1.5 there (dry: not 2.5, which stood 0.5 m wet where neither side was); 2.5 at (9, 4) (0.5 m deep: not 3.5, 1.5 m, deeper than the parent alone). |
| derived_dry_under | M6 | Parent height 4, `a`'s 0 at (8, 0), blend 1; the parent's `level` derived from `height` with a row `max(v, 3)`; `a` declares no `level` | build | `level` 0 at (12, 4) (a dry, not 3 m of the parent's water over `a`'s ground); 4 at (2, 4). |
| derived_lower_head | M6 | derived_lower with a pad (level 4) over (11, 3) to (13, 5); then a row `v + 3` on the parent's `height` under the pad; then back; one store | build; build; build | `level` 4 at (12, 4); with the row, `height` still 4 there and `level` 2.5 (the head view's 7 lowers it, as a fresh build gives: a key on the build view alone kept 4); back, 4. |
| turned_tone | M4 | `a`'s `tone` (unit `ratio`) 3 west of x = 4 and 0 east, at (8, 4), `face = 30` | build | Tone 0.5096 at post (13, 5) and 2.0096 at (13, 6), bilinear (tolerance 0.0001); 3 at (10, 6). |
| turned_wall | M4 | `a` with a 4 m wall between its posts x = 3 and 4, at (8, 4), `face = 30` | build | Height 0 at post (13, 5) and 4 at (13, 6) (each a wall cell's nearest post; bilinear gives 0.68 and 2.68); 4 at (10, 6). |
| field_read | M4 | A parent field `probe` with base `= a.height`; `a` at (8, 0) with height 3 | build | `probe` 3 at a post inside the footprint and 0 outside. |
| datum_same | M4 | Parent and import `datum = "sea"`; then `lift = 2` added | build each | No datum diagnostic, height as the import's; then `import.lift_on_datum`. |
| datum_differs | M4 | Parent `datum = "sea"`, import `datum = "chart"`; then `lift = 2` | build each | `import.datum_differs`, exit 1; then no diagnostic and the import's height 2 m higher in the parent. |
| no_datum | M4 | Neither declares a datum | build | `import.no_datum` warning with `data.write`; exit 0. |
| exit_forgotten | M4 | `a` with an exit `east`, and no parent reference to it | build | `import.exit_unanswered`, subject `a.feature.east`, `data.write` `ends_at = "a.east"`; exit 1. |
| exit_answered | M4 | `exit = ["width"]` on the exit kind, exit width 6; a parent path `ends_at = "a.east"` of width 6; then of width 4; then a deliberate end `on.ref = "a.east"` of width 6 | build each | No diagnostic; then `import.exit_mismatch`, `data.attribute` `width`, exit 1; then no diagnostic. |
| exit_outside | M4 | `a`'s exit at (7, 4) and a footprint of `a`'s west half | build | `import.exit_outside`, subject `a.feature.east`. |
| exit_nested | M4 | `p` imports `m`, `m` imports `n` with an exit `east` that `m` answers; then `m` without the answer | build `p` each | No diagnostic; then `import.exit_unanswered`, subject `m.n.feature.east`. |
| cycle | M4 | `a/world.toml` imports `../world.toml`, which imports `a` | merge | `import.cycle` naming both files; exit 1. |
| parent_anchored | M4 | A shaped import of height 2; a parent anchored feature setting height 5 over part of it | build | Height 5 under the parent feature; 2 elsewhere in the footprint. |
| import_anchored | M4 | An import holding an anchored feature setting height 3; a parent shaped feature setting 9 over it | build | Height 3 there: the import's anchored ground holds. |
| anchored_clash | M4 | The same import; the parent's feature anchored | build | `authority.conflict`, subject the parent feature, `related` the import's feature in `a/world.toml`, `data.leaves` 3 below `data.asks` 9; exit 1. |
| anchored_clash_above | M4 | As `anchored_clash`, the import declared in a layer of rank 1, so its anchored ground is the later entry | build | `authority.conflict`, subject the parent feature, `related` the import's feature; the earlier `over` leaves 9 and the import asks 3 (`data.asks` below `data.leaves`); the hint never asks the import's ground to read the parent; exit 1. |
| layer_rank | M4 | A shaped import of height 2 in the world layer; a shaped parent feature setting 6 in layer `join` rank 1; then `join` at rank -1 | build each | 6 there; then 2. |
| after_dropped | M4 | `[layer.join] after = "imports"` | merge | `doc.unknown_key` at `after`. |
| blend_ground | M4 | `a`'s height -3 west of x = 4 and 2 east; parent height -5; `blend = "= smoothstep(-1, 0, a.height)"` | build | -5 at a post where `a` is -3; 2 at a post where `a` is 2. |
| blend_range | M4 | `blend = "= 2"` | build | `formula.range`; exit 1. |
| seam | M4 | A hard edge: import height 4 on parent height 0; then `max_grade = 5`; then a parent anchored feature over the whole edge | build each | `import.seam` with `data.grade` 4; then none; then none. |
| place_blended | M4 | `blend = "= smoothstep(0, 4, d)"`; `footprint` the import's east 6 m; a parent path `ends_at = "a.quay"` with `a.quay` 1 m inside the import's extent, outside the footprint | build | `import.place_blended`, `data.weight` 0 (a place inside the footprint is held, 6.7). |
| place_held | M6 | Parent height 0, `a`'s 4 at (32, 0), 48 by 64; `blend = "= smoothstep(0, 16, d)"`; a parent path `ends_at = "a.gate"`, `a.gate` on `a`'s west edge at (0, 32) | build | Height 4 at (32, 32) and (44, 32) (held: the blend alone gives 0 and 3.375); 0 at (32, 0), 32 m from the gate; no `import.place_blended`. |
| field_match | M4 | `a` with a field `moss` the parent lacks; then the parent declares `moss` with `role = "keep_out"` and `a`'s `moss` has none | build each | `import.field_unmatched` with `data.field` `moss`; then `import.field_differs`, exit 1. |
| overlap | M4 | Imports `a`, `b`, footprints overlapping by 2 m by 8 m | build | `import.overlap`, `data.area` 16. |
| outside | M4 | `a` placed 4 m past the parent's east edge | build | `import.outside`; the build holds no `a.` instance past the edge. |
| coarser | M4 | Parent resolution 2, import resolution 1 | build | `import.coarser`, `data.ratio` 2. |
| binding | M4 | The import's own pack binding `box_small` to a 1 m box | build | `import.binding_differs`, `data.id` `box_small`; exit 1. |
| import_reserved | M4 | `[import.feature]` | merge | `identity.reserved`, subject `import.feature`, `data.nearest` `feature_2`; exit 1. |
| binding_material | M4 | Parent and `a` each paint `moss` on their `material` field; the two packs bind `moss` with different colours | build | `import.binding_differs`, `data.id` `moss`; exit 1. |
| brought_unknown_material | M4 | `a` paints `moss` on the `material` field; `a`'s pack binds `moss`, the parent's binds only `sand` | build | `pack.unknown_material`, subject `import.a`; exit 1. |
| diag_scoped | M4 | `a` with a feature of an unknown kind | build | `authority.unknown_kind`, subject `a.feature.bad`, file `a/world.toml`, chain through import `a`; exit 1. |
| import_changes | M4 | `changes = ["high"]`, `a`'s change `high` setting height 4; then `changes = ["hgih"]` | build each | The parent's height 4 in the footprint; then `doc.unknown_change` nearest `high`. |
| give_fill | M4 | A fill `town` with `[given.land]` (field, default `= 0`) and `height = land + 1`; a parent of height `= 0.5 * x` giving `land = "height"` | build the town alone; build the parent | Alone, the town's height 1; in the parent, `0.5 * x + 1` inside the footprint. |
| give_moved | M4 | `give_fill`'s parent, then a change making its height `= 0.25 * x` | build the parent; build the change against it on one store | The town's height follows the parent's (7 then 4 at x = 12): the `given` node's edge to the parent's field (`graph.md` 2, "give"), checked with every other row by the model's test `every_reference_in_graph_md_2_makes_an_edge`. |
| give_errors | M4 | `give = { lnad = "height" }`; then `give = { land = "zone" }` with `zone` a region | merge each | `import.given_unknown` nearest `land`; then `import.given_type`. |
| give_outside | M4 | A given field and the fill placed 4 m past the parent's edge | build | `import.give_outside`; exit 1. |
| give_dropped | M4 | A given feature with an attribute `tone` written, which the fill's kind does not declare | build | `import.given_dropped`, `data.attribute` `tone`. |
| give_cycle | M4 | A parent region given to `town` as `limits`, written `where = "= dist('town.edge', 8) < 2"`, with `town.edge` following `limits` | build | `graph.cycle`; exit 1. |
| give_reads_fill | M4 | The same, the limits drawn from `town.arterial`, a path that reads no given, while `town.edge` follows `limits` | build | No `graph.cycle`; exit 0. |
| moved_square | M4 | Two districts (their kind has a default `tone`, written on neither) and a square in each, given to a fill whose worker step has `for_each = "given:districts"` and reads `given:squares` within its item; a change moving one square inside its district | build; build with the change | No diagnostic (a default is not handed down); second: `town.step:lay/i:districts.d1` ran, `town.step:lay/i:districts.d2` did not. |
| join_step | M4 | A generate step in layer `join` (rank 1) reading `height` and `a.height`, returning field `own` = `a.height`; then the step in a layer of rank -1 | build each | `look#own` equals `a`'s height inside the footprint; then `graph.read_outside`. |
| join_below | M4 | `a` (height 2) declared in layer `top` (rank 2); a generate step in layer `join` (rank 1) returning `seen` = `height` | build | `look#seen` 0 at post (12, 4), the build's `height` 2 there: the step's stratum leaves out an import of a higher layer. |
| nested_names | M4 | `p` imports `m` at (8, 0); `m` imports `n` at (2, 0) holding instance `hut` at (1, 1); a bookmark `on.ref = "m.n.hut"` | build | `m.n.hut` at (11, 1). |
| placement_keys | M4 | An import with `at` and `on`; then neither | merge each | `doc.placement_twice`; then `doc.missing_key`, subject `import.a`. |
| given_default | M4 | A fill's `[given.land]` `accepts = "field"` with no `default`; then `number`; then `region` | merge each | `doc.missing_key`; again; then no diagnostic. |
| given_read | M4 | A fill of height `= if(inside('limits'), density, 0)` with a given region `limits` and number `density` (default 1); the parent gives its west region and `density = 3` | build the fill alone; build the parent | Alone 1; in the parent 3 inside the region, 0 outside it. |
| footprint_ref | M4 | A footprint of points (4 m by 8 m at (8, 4)); a parent field `= if(inside('a.footprint'), 1, 0)`; a feature `follows = { ref = "a.footprint" }` | build | 1 inside, 0 outside; exit 0. |
| instance_outside | M4 | `a`'s `hut` at (2, 2) and `shed` at (6, 6), footprint the west half | build | `a.hut` brought; no `a.shed`. |
| imported_twice | M4 | One file imported as `a` at (0, 0) and `b` at (16, 0) | build | No `import.cycle`; `a.hut` at (2, 2), `b.hut` at (18, 2). |
| blend_self | M4 | `blend = "= smoothstep(0, 1, f('height'))"`, `height` brought | build | `graph.cycle`; exit 1. |
| seam_edge | M4 | An import ramping to 2 m along its east side, which lies on the parent's east extent edge; then 2 m higher on its west side too | build each | No diagnostic; then `import.seam`, `data.grade` 2. |
| lift_partial | M4 | The import declares no datum, `lift = 2`; its surface 3 and a field `moss` (no role) 1 | build | No diagnostic; surface 5; `moss` 1. |
| exit_passed | M4 | `m` passes `n`'s exit on with its own exit placed `on` it; `p` answers `m.door`; then does not | build each | No diagnostic; then `import.exit_unanswered`, subject `m.feature.door`. |
| binding_same | M4 | The import's pack binds `box_small` as the parent's does; then to a 1 m box | build each | No diagnostic; then `import.binding_differs`. |
| change_scoped | M4 | A parent change file with `[feature."a.quay"]` | build with the change | `identity.bad_id`; exit 1. |
| given_part | M4 | A given path `r1` under `routes`; the fill's path `starts_at = "routes.r1.end"` | build | `town.stop` starts at (10, 2). |
| import_kind_surface | M6 | `a` (8 m square, no datum) holds a deck `span` (top 3 m, path (1, 4) to (7, 4), half-width 1) and a hollow `cave` (floor -1 m, `cut` 3, region (1, 1) to (3, 3)), a lamp (`surface = "span.top"`) and a bookmark `mid` on the deck; placed at (20, 8), `face = 90`, `lift = 2`; a parent point `view` at (24, 3) with `surface = "a.span.top"`; then `a` and `view` moved 8 m west on one store; then a parent path `ends_at = "a.span.end"` | build; query `surfaces_at`; build; build | The manifest lists meshes `a.cave` and `a.span`; cut quads at posts (21, 5) and (22, 6), none at (23, 5) or (20, 5); the lamp's `z` 5; `a.mid` at (24, 4, 5) and `view` at (24, 3, 5); at (24, 4) `surfaces_at` gives `height` 2 and `a.span.top` 5; moved, cut at (13, 5) and (14, 6), none at (21, 5), `a.mid` at (16, 4, 5); then `surface.gap`, `data.gap` 3000 (the path's end at 2 m). |
| import_nested_stand | M6 | p imports m at (4, 4), no lift; m is import_kind_surface's parent (a lamp of `a` on `a.span.top`) | build | `m.a.lamp`'s `z` 5 (on the deck brought twice), not the ground's. |
| brought_asset | M6 | `a`'s own pack binds `heavy` (1000 triangles), the parent's binds only `light`; `[budget] triangles = 2000`; then 500 | build; build | The manifest's `assets.heavy.pack` is `a` and chunk [0, 0] counts 1010 triangles; then `build.over_budget` naming `heavy`, exit 1. |

Rules of this chapter with no case yet, owed by M4 slice 3 (each row able to fail): a footprint
naming a parent region (stays where it is when the import moves); `authority.squeezed` where the
import's anchored cover fades; a moved fill re-running its given readers; `brought` and parent field
chunks reused away from the old and new footprint; change report causes in the parent's terms
(`import.b.at`) checked in `changes.json`. Written in M4 slice 2: the thirteen rows from
`placement_keys` to `given_part` above.

## Changed from the spike

| Spike | v1 | Why |
|---|---|---|
| `[[part]]`, master | `[import.<id>]`, parent | "Part" means a building part (`vocabulary.md` 6). |
| `apron` and a fixed falloff | `blend`, default 1; a seam check | The falloff bent both sides by a number nobody chose. |
| `outer` (the master bends) and `apron` (the part bends) | Authority, two entries per field | One question, answered as inside one world. |
| `lift` found by eye | `datum`; `lift` only between different or undeclared datums | `atlas`'s nine lifts. |
| `after = "parts"` | Rank above the import's layer | One mechanism, not two. |
| `expects = "sea"` | Exits, `datum`, and the seam check | What a part expected was never compared. |
| `ground_as` renaming materials | `import.binding_differs`: one binding per name | No silent winner. |
| `wild.edge.2` | `wild.footprint` and `follows` with `span` | A number breaks when the outline gains a point. |
| `joins_at` | An exit, answered by name | The parent names places. |
| `portal` | An import placed `on` a mount (M5) | Engines use portal for culling. |
| A part given anything rebuilt whole | `given` nodes; a step per item | TODO "rebuild a part by area". |
| `part.overruled` warned and cleared nothing | `authority.conflict` or `authority.squeezed` | Principle 3. |
| Noise of a part seeded by the master | Its own seed and frame | Moving a part changed its noise. |

## Open

| Question | Decided by |
|---|---|
| Selecting an import's features by kind from the parent (`'a/kind:x'`, or a give-like export). | The first world that needs one; `atlas`, M6. |
| Naming the sides of a footprint by what they face (TODO: "the sides facing part X"), beyond `follows` with `span`. | `border`'s join layer in M4, if `span` fractions break when the outline changes. |
| A filter when a fine import is read onto coarse posts. | The first world where `import.coarser` shows lost detail on foot. |
| `max_grade`'s default 0.5. | The M4 worlds' seam measures (`strait`, `border`, `county`). |
| An import's own rule growing past its edge, fading (TODO). | `border`'s fringe in M4; until then a parent scatter in the join layer. |
| A parent that offers what a part may take without the part declaring it. | Not planned: R11 makes it opt-in per input. |
| Two exits that meet at one point with nothing between them. | A library kind placed `on` one naming the other answers both today; revisited with `atlas`. |
| What leaves undeclared (R9: "what leaves is declared as an exit"): an import path cut by the footprint, or reaching the import's extent, with no exit there. A warning `import.exit_undeclared` per cut end, or nothing. | `atlas`'s river that stops at the edge, M4 slice 2: if it goes unseen on foot, the warning. |
| A parent naming an import's produced face or cell as a place (`a.<network>.<face>`, R13 across imports). | The first parent that must place on an import's generated block; `county` in M4. |
| A brought kind surface where the parent's own surfaces only are read (6.1): a parent scatter's `surface`, a parent point `on` a brought feature by its name, the level test (`rules.md` 8.1); and whether a footprint narrower than the import clips its surfaces (today they come whole, with their feature). | The first parent that scatters on, or places by `on` onto, an import's deck; a footprint that cuts a bridge. |
| Matching a field by role as well as by name (an import's `surface` field named `ground` is not brought today, only warned). | The first world pair whose surface fields differ by name. |
