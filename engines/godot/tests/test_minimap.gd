extends SceneTree
## The loader's minimap (addons/via_loader/minimap.gd; spec/build.md 15.1, "The minimap"), headless:
##   godot --headless --path engines/godot --script res://tests/test_minimap.gd -- --build DIR \
##         [--png FILE]
## DIR is a build with a player map export, with floors (station) or without (atlas). Checks that the map is read from
## the manifest's `player_map` export; the ground image is the map's size, north row at the top,
## each cell its class's colour; the floor for a height is the highest floor starting at or below
## it; the corner map puts the walker at its centre and what lies ahead of it above the centre, so
## it turns with the walker, at CORNER_METRES across (the world's side when less); the whole map fits the extent, north up; the
## labels shown are the floor's; and a build with no export says so. --png draws the corner map and
## the whole map into FILE (needs a display; headless draws nothing).

const Minimap := preload("res://addons/via_loader/minimap.gd")
const Npy := preload("res://addons/via_loader/npy.gd")
var failed := 0


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


static func labels_of(labels: Array, n: int) -> int:
	return labels.filter(func(l): return l.get("floor") != null and int(l.floor) == n).size()


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var dir := ""
	var png := ""
	var k := 0
	while k + 1 < args.size():
		match args[k]:
			"--build": dir = args[k + 1]
			"--png": png = args[k + 1]
		k += 2
	var map := Minimap.new()
	var err := map.read(dir)
	check("the player map is read from the manifest's player_map export", err == "" and map.folder.contains("/export/"), err)
	if err != "":
		print("FAIL: %d check(s) failed" % failed)
		quit(1)
		return
	var index: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(map.folder + "/map.json"))
	var size: Array = index.transform.size
	check("the ground image is the map's size", map.ground_image.get_width() == int(size[0]) and map.ground_image.get_height() == int(size[1]),
		"%dx%d against %s" % [map.ground_image.get_width(), map.ground_image.get_height(), size])
	# every cell its class's colour, the file's row 0 (south) at the image's bottom
	var arr := Npy.read(map.folder + "/ground.npy")
	var cols := int(size[0])
	var rows := int(size[1])
	var wrong := 0
	var seen := {}
	for r in rows:
		for c in cols:
			var id: int = arr.bytes[r * cols + c]
			seen[id] = true
			var want: Color = map.colours[id]
			var got := map.ground_image.get_pixel(c, rows - 1 - r)
			if absf(got.r - want.r) > 0.01 or absf(got.g - want.g) > 0.01 or absf(got.b - want.b) > 0.01 or absf(got.a - want.a) > 0.01:
				wrong += 1
	check("each cell is its class's colour, the south row at the bottom", wrong == 0, "%d cells wrong; %d classes seen" % [wrong, seen.size()])
	var named := seen.keys().map(func(i): return map.classes[i])
	check("the map holds ground and footprint classes", named.any(func(n): return n.begins_with("ground:"))
		and named.any(func(n): return n == "footprint"), str(named))
	# ways: every way of vectors.json not `under` is drawn as its line (build.md 15.1, M7)
	var vec: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(map.folder + "/vectors.json"))
	var want_ways: int = vec.ways.filter(func(w): return not w.get("under", false)).size()
	check("each way not under is read to be drawn as its line", map.ways.size() == want_ways and want_ways > 0,
		"%d of %d" % [map.ways.size(), want_ways])
	# floors: the highest starting at or below the feet. A map with no floors (atlas) draws the
	# ground plan at every height; with floors, below the lowest none, at each floor's start that
	# floor, and between two starts the lower.
	var listed: Array = index.get("floors", [])
	check("every floor of map.json read", map.floors.size() == listed.size(), "%d of %d" % [map.floors.size(), listed.size()])
	var lows: Array = map.floors.map(func(f): return float(f.z[0]))
	var idx: Array = map.floors.map(func(f): return int(f.index))
	var expect := []
	if lows.is_empty():
		expect = [[-1000.0, -1], [0.0, -1], [1000.0, -1]]
	else:
		expect.append([lows.min() - 0.01, -1])
		for i in lows.size():
			expect.append([lows[i], idx[i]])
			var above: Array = lows.filter(func(z): return z > lows[i])
			expect.append([(lows[i] + above.min()) / 2.0 if not above.is_empty() else lows[i] + 20.0, idx[i]])
	for e in expect:
		check("feet at %.3f m stand on floor %d" % [e[0], e[1]], map.floor_at(e[0]) == e[1], "%d" % map.floor_at(e[0]))
	# the corner map turns with the walker
	get_root().add_child(map)
	map.size = Vector2(Minimap.CORNER, Minimap.CORNER)
	map.full = false
	var c0 := Vector2(Minimap.CORNER, Minimap.CORNER) / 2.0
	var at := Vector2(30.0, 34.0) if map.extent.has_point(Vector2(30.0, 34.0)) else map.extent.get_center()
	for h in [Vector2(0, 1), Vector2(1, 0), Vector2(-1, -1).normalized()]:
		map.set_walker(at, h, 1.0)
		var centre := map.to_screen(at)
		var ahead := map.to_screen(at + h * 10.0)
		var right := map.to_screen(at + Vector2(h.y, -h.x) * 10.0)
		var px := 10.0 * Minimap.CORNER / minf(Minimap.CORNER_METRES, maxf(map.extent.size.x, map.extent.size.y))
		check("heading %s: the walker at the centre, 10 m ahead straight up and 10 m to its right straight right, %.1f px each" % [h, px],
			centre.distance_to(c0) < 1e-3 and ahead.distance_to(c0 + Vector2(0, -px)) < 1e-3 and right.distance_to(c0 + Vector2(px, 0)) < 1e-3,
			"%s %s %s" % [centre, ahead, right])
	# the whole map: north up, the extent fitted
	map.full = true
	map.size = Vector2(800, 600)
	var sw := map.to_screen(map.extent.position)
	var ne := map.to_screen(map.extent.end)
	var fit := minf(800.0 / map.extent.size.x, 600.0 / map.extent.size.y)
	check("the whole map: north up, the extent fitted and centred", sw.x < ne.x and sw.y > ne.y
		and absf((ne.x - sw.x) - map.extent.size.x * fit) < 1e-3 and absf((sw.y - ne.y) - map.extent.size.y * fit) < 1e-3
		and absf((sw.x + ne.x) / 2.0 - 400.0) < 1e-3 and absf((sw.y + ne.y) / 2.0 - 300.0) < 1e-3,
		"south-west %s, north-east %s" % [sw, ne])
	# labels: on a floor, only that floor's and the ground plan's other than rooms; on the ground
	# plan (no floor) the ground plan's
	var bad := []
	var heights: Array = [lows.min() - 1.0] if not lows.is_empty() else [0.0]
	heights.append_array(lows)
	for z in heights:
		map.set_walker(at, Vector2(0, 1), z)
		var n := map.floor_at(z)
		for l in map.shown_labels():
			var fl = l.get("floor")
			if not ((fl == null and (n < 0 or String(l.get("class")) != "room")) or (fl != null and int(fl) == n)):
				bad.append("%s at %.2f" % [l.key, z])
		var want: int = labels_of(map.labels, n)
		if map.shown_labels().filter(func(l): return l.get("floor") != null).size() != want:
			bad.append("floor %d shows %d of its %d" % [n, map.shown_labels().size(), want])
	check("labels: a floor's own and the ground plan's other than rooms; every one of the floor's", bad.is_empty() and not map.labels.is_empty(), str(bad))
	var none := Minimap.new()
	check("a build with no player_map export says so", none.read("res://tests/fixture_build".replace("res://", ProjectSettings.globalize_path("res://"))) != "")
	none.free()
	if png != "":
		map.full = false
		map.layout()
		map.set_walker(Vector2(33.0, 35.0), Vector2(1, 1).normalized(), 1.0)
		for f in 3:
			await RenderingServer.frame_post_draw
		get_root().get_texture().get_image().save_png(png)
		map.set_full(true)
		map.set_walker(Vector2(33.0, 35.0), Vector2(1, 1).normalized(), lows[1] + 0.5 if lows.size() > 1 else 0.0)
		for f in 3:
			await RenderingServer.frame_post_draw
		get_root().get_texture().get_image().save_png(png.get_basename() + "_full.png")
	print("%s: %d check(s) failed" % ["PASS" if failed == 0 else "FAIL", failed])
	quit(0 if failed == 0 else 1)
