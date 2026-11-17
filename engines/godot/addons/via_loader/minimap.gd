extends Control
## The player map in the loader (spec/build.md 15.1, "The minimap", M6 close): the map library's
## export (build.md 9.4) drawn as a corner map that turns with the walker, and as the whole map,
## north up, at a toggle. Read from the build's manifest alone: the first entry of `exports` whose
## `output` is `player_map`, its `map.json`, `ground.npy`, `vectors.json`, `labels.json` and each
## floor's plan. Draws the ground classes, then the plan of the floor the walker's feet are on (the
## highest floor whose start is at or below them, build.md 9.4), that plan's labels (the ground's
## off a floor), and the walker.
##
##   var map := Minimap.new()
##   var err := map.read(build_dir)      # "" when read; the reason otherwise
##   canvas_layer.add_child(map)
##   map.set_walker(Vector2(x, y), Vector2(dx, dy), feet_z)   # every frame
##   map.set_full(true)                   # the whole map (M toggles it, `toggle_key`)

const Npy := preload("res://addons/via_loader/npy.gd")

## Ours: the corner map is CORNER pixels a side and shows CORNER_METRES across (the world's larger
## side when that is less), so a street and the houses either side of it read at a glance; MARGIN
## pixels from the window's top right corner. Labels are drawn in order (regions, POIs, labels,
## rooms, as lib/map placed them) and one whose text would meet one drawn is left out at that
## scale.
const CORNER := 240
const CORNER_METRES := 160.0
const MARGIN := 16
## Ours: the colours a class is drawn in when the build says none: a way, a footprint, water, and
## a ground class whose material the manifest does not colour.
const WAY := Color(0.98, 0.95, 0.86)
## Ours (M7): a way is also drawn from vectors.json as its line, WAY over a CASING one pixel wider
## each side, never less than WAY_PX across, so a road narrower than a pixel of the whole map is
## one line, not dashes where its cells fall between the pixels (atlas's earth roads, M6 close).
const CASING := Color(0.45, 0.33, 0.22)
const WAY_PX := 1.5
const FOOTPRINT := Color(0.36, 0.33, 0.31)
const WATER := Color(0.25, 0.45, 0.62)
const GROUND := Color(0.6, 0.6, 0.55)
const FLOOR := Color(0.93, 0.91, 0.86, 0.85)
const WALL := Color(0.15, 0.14, 0.13)
const ROOM := Color(0.35, 0.3, 0.25)
const LINK := Color(0.75, 0.3, 0.2)
const INK := Color(0.1, 0.1, 0.1)
const WALKER := Color(0.85, 0.1, 0.1)

var full := false
var show_walker := true   # the walker's arrow; the sheet's whole map (map.png) leaves it out
var toggle_key := KEY_M
var folder := ""          # the export's folder, absolute
var extent := Rect2()     # world metres
var cell := 1.0
var classes: Array = []   # class names by id
var colours: Array[Color] = []
var ground_image: Image   # one pixel per map cell, north at the top
var floors: Array = []    # each { "index", "z": [lo, hi or null], "plan": Dictionary }
var labels: Array = []
var ways: Array = []      # vectors.json's ways not `under`: { "width", "points" }
var walker := Vector2.ZERO
var heading := Vector2(0, 1)
var feet := 0.0
var _tex: ImageTexture


func read(build_dir: String) -> String:
	## Reads the build's player map. "" when read, else why not (no export, a file missing or bad).
	var manifest = JSON.parse_string(FileAccess.get_file_as_string(build_dir + "/manifest.json"))
	if not manifest is Dictionary:
		return "no manifest.json"
	var exports = manifest.get("exports")
	var names: Array = exports.keys() if exports is Dictionary else []
	names.sort()
	for n in names:
		var e = exports[n]
		if e is Dictionary and e.get("output") == "player_map" and e.get("folder") is String:
			folder = build_dir + "/" + String(e.folder).trim_suffix("/")
			break
	if folder == "":
		return "the build has no player_map export"
	var index = JSON.parse_string(FileAccess.get_file_as_string(folder + "/map.json"))
	if not index is Dictionary or index.get("format") != "via-map-0":
		return "map.json is not via-map-0"
	var t: Dictionary = index.get("transform", {})
	var ex: Array = t.get("extent", [0, 0, 1, 1])
	extent = Rect2(ex[0], ex[1], float(ex[2]) - float(ex[0]), float(ex[3]) - float(ex[1]))
	cell = float(t.get("cell", 1.0))
	var materials: Dictionary = manifest.get("materials", {})
	classes.clear()
	colours.clear()
	for c in index.get("classes", []):
		classes.append(String(c.name))
		colours.append(class_colour(String(c.name), materials))
	var layers: Dictionary = index.get("layers", {})
	var arr := Npy.read(folder + "/" + String(layers.get("ground", "ground.npy")))
	var size: Array = t.get("size", [0, 0])
	if Npy.expect(arr, "|u1", [int(size[1]), int(size[0])]) != "":
		return "ground.npy is not |u1 of the map's size"
	ground_image = ground_of(arr.bytes, int(size[0]), int(size[1]), colours)
	_tex = ImageTexture.create_from_image(ground_image)
	var vec = JSON.parse_string(FileAccess.get_file_as_string(folder + "/" + String(layers.get("vectors", "vectors.json"))))
	ways = []
	if vec is Dictionary:
		for w in vec.get("ways", []):
			if not w.get("under", false) and w.get("points") is Array:
				ways.append({"width": float(w.get("width", 1.0)), "points": w.points})
	var l = JSON.parse_string(FileAccess.get_file_as_string(folder + "/" + String(layers.get("labels", "labels.json"))))
	labels = l.get("labels", []) if l is Dictionary else []
	floors.clear()
	for f in index.get("floors", []):
		var plan = JSON.parse_string(FileAccess.get_file_as_string(folder + "/" + String(f.file)))
		if not plan is Dictionary:
			return "%s is not a floor plan" % f.file
		floors.append({"index": int(f.index), "z": f.z, "plan": plan})
	return ""


static func class_colour(name: String, materials: Dictionary) -> Color:
	## A ground class's colour: a material's own (`ground:<material>`, its linear `colour` shown
	## as sRGB), water, a way, a footprint; `none` is clear.
	if name == "none":
		return Color(0, 0, 0, 0)
	if name.begins_with("ground:"):
		var c = materials.get(name.substr(7), {}).get("colour")
		return Color(c[0], c[1], c[2]).linear_to_srgb() if c is Array and c.size() >= 3 else GROUND
	if name == "water" or name.begins_with("water:"):
		return WATER
	if name.begins_with("way:"):
		return WAY
	if name == "footprint":
		return FOOTPRINT
	return GROUND


static func ground_of(cells: PackedByteArray, cols: int, rows: int, palette: Array[Color]) -> Image:
	## The ground classes as an RGBA8 image, one pixel a cell, the north row at the top (the
	## file's row 0 is the south row).
	var lut := PackedByteArray()
	lut.resize(256 * 4)
	for k in 256:
		var c: Color = palette[k] if k < palette.size() else Color(1, 0, 1)
		lut[4 * k] = c.r8
		lut[4 * k + 1] = c.g8
		lut[4 * k + 2] = c.b8
		lut[4 * k + 3] = c.a8
	var px := PackedByteArray()
	px.resize(cols * rows * 4)
	for r in rows:
		var src := r * cols
		var dst := (rows - 1 - r) * cols * 4
		for c in cols:
			var k := cells[src + c] * 4
			px[dst + 4 * c] = lut[k]
			px[dst + 4 * c + 1] = lut[k + 1]
			px[dst + 4 * c + 2] = lut[k + 2]
			px[dst + 4 * c + 3] = lut[k + 3]
	return Image.create_from_data(cols, rows, false, Image.FORMAT_RGBA8, px)


func floor_at(z: float) -> int:
	## The floor whose plan is drawn for feet at z: the highest whose start is at or below z; -1
	## (the ground plan alone) without floors or below the lowest.
	var best := -1
	var start := -INF
	for f in floors:
		var lo := float(f.z[0])
		if lo <= z and lo >= start:
			best = int(f.index)
			start = lo
	return best


func set_walker(p: Vector2, facing: Vector2, z: float) -> void:
	walker = p
	if facing.length() > 1e-6:
		heading = facing.normalized()
	feet = z
	queue_redraw()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	clip_contents = true
	layout()


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == toggle_key:
		set_full(not full)


func set_full(on: bool) -> void:
	full = on
	layout()


func layout() -> void:
	## The whole window for the whole map; else a CORNER square MARGIN in from the top right.
	if full:
		set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	else:
		set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT)
		offset_left = -CORNER - MARGIN
		offset_right = -MARGIN
		offset_top = MARGIN
		offset_bottom = MARGIN + CORNER
	queue_redraw()


func view() -> Rect2:
	## The control's own rectangle, which clips what is drawn (clip_contents).
	return Rect2(Vector2.ZERO, size)


func to_screen(w: Vector2) -> Vector2:
	## World metres to screen pixels. The corner map: the walker at its centre, its heading up,
	## CORNER_METRES across. The whole map: north up, the extent fitted into the control.
	var r := view()
	if full:
		var s := minf(r.size.x / extent.size.x, r.size.y / extent.size.y)
		var d := w - extent.get_center()
		return r.get_center() + Vector2(d.x, -d.y) * s
	var s := CORNER / corner_metres()
	var turn := PI / 2.0 - heading.angle()   # the heading turned to north
	var d := (w - walker).rotated(turn)
	return r.get_center() + Vector2(d.x, -d.y) * s


func corner_metres() -> float:
	return minf(CORNER_METRES, maxf(extent.size.x, extent.size.y)) if extent.size.x > 0.0 else CORNER_METRES


func shown_labels() -> Array:
	## The labels of the plan drawn: the current floor's, else the ground plan's.
	var n := floor_at(feet)
	return labels.filter(func(l): return (l.get("floor") == null and n < 0) or (l.get("floor") != null and int(l.floor) == n) \
		or (l.get("floor") == null and n >= 0 and String(l.get("class")) != "room"))


func _draw() -> void:
	if _tex == null:
		return
	var r := view()
	draw_rect(r, INK)
	# The ground image: its pixel (0, 0) is the cell at the extent's north-west corner.
	var o := to_screen(Vector2(extent.position.x, extent.end.y))
	var ax := to_screen(Vector2(extent.position.x + cell, extent.end.y)) - o
	var ay := to_screen(Vector2(extent.position.x, extent.end.y - cell)) - o
	draw_set_transform_matrix(Transform2D(ax, ay, o))
	draw_texture(_tex, Vector2.ZERO)
	draw_set_transform_matrix(Transform2D.IDENTITY)
	_draw_ways(r)
	var n := floor_at(feet)
	for f in floors:
		if int(f.index) == n:
			_draw_plan(f.plan)
	var font := ThemeDB.fallback_font
	var taken: Array[Rect2] = []
	for l in shown_labels():
		var at: Array = l.at
		var p := to_screen(Vector2(at[0], at[1]))
		var box := Rect2(p + Vector2(3, -15), font.get_string_size(String(l.text), HORIZONTAL_ALIGNMENT_LEFT, -1, 12) + Vector2(0, 4))
		if not r.encloses(box) or taken.any(func(t): return t.intersects(box)):
			continue
		taken.append(box)
		draw_string(font, p + Vector2(3, -3), String(l.text), HORIZONTAL_ALIGNMENT_LEFT, -1, 12, INK)
	if show_walker:
		var w := to_screen(walker)
		var h := Vector2(0, -1) if not full else Vector2(heading.x, -heading.y)
		var side := Vector2(-h.y, h.x)
		draw_colored_polygon(PackedVector2Array([w + h * 9.0, w - h * 5.0 + side * 6.0, w - h * 5.0 - side * 6.0]), WALKER)
	draw_rect(r, INK, false, 2.0)


func pixels_a_metre() -> float:
	var r := view()
	if full:
		return minf(r.size.x / extent.size.x, r.size.y / extent.size.y)
	return CORNER / corner_metres()


func _draw_ways(r: Rect2) -> void:
	## Every way's line, casings first so a crossing reads as one: its width at the map's scale,
	## at least WAY_PX. Lines wholly outside the view are skipped.
	var s := pixels_a_metre()
	var lines: Array = []
	for w in ways:
		var line := PackedVector2Array()
		var lo := Vector2(INF, INF)
		var hi := Vector2(-INF, -INF)
		for q in w.points:
			var p := to_screen(Vector2(q[0], q[1]))
			line.append(p)
			lo = lo.min(p)
			hi = hi.max(p)
		if line.size() >= 2 and Rect2(lo, hi - lo).grow(8.0).intersects(r):
			lines.append([line, maxf(w.width * s, WAY_PX)])
	for e in lines:
		draw_polyline(e[0], CASING, e[1] + 2.0)
	for e in lines:
		draw_polyline(e[0], WAY, e[1])


func _draw_plan(plan: Dictionary) -> void:
	for s in plan.get("surfaces", []):
		_fill(s.get("pieces", []), FLOOR)
	for room in plan.get("rooms", []):
		_outline(room.get("pieces", []), ROOM, 1.0)
	for w in plan.get("walls", []):
		var ring := PackedVector2Array()
		for q in w.get("outline", []):
			ring.append(to_screen(Vector2(q[0], q[1])))
		if ring.size() >= 3:
			draw_colored_polygon(ring, WALL)
	for d in plan.get("doors", []):   # a door is the line across its opening
		var line := PackedVector2Array()
		for q in d.get("points", []):
			line.append(to_screen(Vector2(q[0], q[1])))
		if line.size() >= 2:
			draw_polyline(line, FLOOR, 3.0)
	for k in plan.get("links", []):
		var line := PackedVector2Array()
		for q in k.get("points", []):
			line.append(to_screen(Vector2(q[0], q[1])))
		if line.size() >= 2:
			draw_polyline(line, LINK, 2.0)


func _fill(pieces: Array, colour: Color) -> void:
	for polygon in pieces:
		if polygon.is_empty():
			continue
		var ring := PackedVector2Array()
		for q in polygon[0]:
			ring.append(to_screen(Vector2(q[0], q[1])))
		if ring.size() >= 3 and not Geometry2D.triangulate_polygon(ring).is_empty():
			draw_colored_polygon(ring, colour)


func _outline(pieces: Array, colour: Color, width: float) -> void:
	for polygon in pieces:
		for ring in polygon:
			var line := PackedVector2Array()
			for q in ring:
				line.append(to_screen(Vector2(q[0], q[1])))
			if line.size() >= 2:
				line.append(line[0])
				draw_polyline(line, colour, width)
