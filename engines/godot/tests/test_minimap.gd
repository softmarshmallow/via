extends SceneTree
## The loader's minimap (addons/via_loader/minimap.gd; spec/build.md 15.1, "The minimap"), headless:
##   godot --headless --path engines/godot --script res://tests/test_minimap.gd -- --build DIR \
##         [--png FILE]
## DIR is a build with a player map export and floors (station). Checks that the map is read from
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
	check("the player map is read from the manifest's player_map export", err == "" and map.folder.ends_with("export/map"), err)
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
	check("the station's map holds ground, way and footprint classes", named.any(func(n): return n.begins_with("ground:"))
		and named.any(func(n): return n == "footprint"), str(named))
	# floors: the highest starting at or below the feet
	check("four floors read", map.floors.size() == 4, "%d" % map.floors.size())
	if map.floors.size() < 4:
		print("FAIL: %d check(s) failed (the rest need station's four floors)" % failed)
		quit(1)
		return
	var lows: Array = map.floors.map(func(f): return float(f.z[0]))
	var expect := [[lows[0] - 0.01, -1], [lows[0], 0], [1.0, 0], [lows[1] + 0.01, 1], [lows[2] + 0.5, 2], [lows[3] + 20.0, 3]]
	for e in expect:
		check("feet at %.3f m stand on floor %d" % [e[0], e[1]], map.floor_at(e[0]) == e[1], "%d" % map.floor_at(e[0]))
	# the corner map turns with the walker
	get_root().add_child(map)
	map.size = Vector2(Minimap.CORNER, Minimap.CORNER)
	map.full = false
	var c0 := Vector2(Minimap.CORNER, Minimap.CORNER) / 2.0
	var at := Vector2(30.0, 34.0)
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
	check("the whole map: north up, the extent fitted and centred", sw.x < ne.x and sw.y > ne.y
		and absf((ne.x - sw.x) - 600.0) < 1e-3 and absf((sw.y - ne.y) - 600.0) < 1e-3 and absf((sw.x + ne.x) / 2.0 - 400.0) < 1e-3,
		"south-west %s, north-east %s" % [sw, ne])
	# labels: a floor's rooms on that floor only
	map.set_walker(at, Vector2(0, 1), lows[1] + 0.5)
	var on1: Array = map.shown_labels().filter(func(l): return l.get("floor") != null).map(func(l): return int(l.floor))
	map.set_walker(at, Vector2(0, 1), 1.0)
	var on0: Array = map.shown_labels().filter(func(l): return l.get("class") == "room").map(func(l): return String(l.key))
	check("labels: on floor 1 only floor 1's, on floor 0 its rooms", not on1.is_empty() and on1.all(func(f): return f == 1)
		and on0.has("kitchen") and on0.has("bunks"), "%s; %s" % [on1, on0])
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
		map.set_walker(Vector2(33.0, 35.0), Vector2(1, 1).normalized(), lows[1] + 0.5)
		for f in 3:
			await RenderingServer.frame_post_draw
		get_root().get_texture().get_image().save_png(png.get_basename() + "_full.png")
	print("%s: %d check(s) failed" % ["PASS" if failed == 0 else "FAIL", failed])
	quit(0 if failed == 0 else 1)
