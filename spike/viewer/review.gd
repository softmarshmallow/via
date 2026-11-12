extends RefCounted
## The modes nobody steers: bookmark renders, the frame-rate bench, the map capture and the
## scripted walk test. Each runs to its end and returns; the caller quits. Only one runs per launch.

const WorldData := preload("res://world_data.gd")
const Bookmarks := preload("res://bookmarks.gd")
const Grass := preload("res://grass.gd")
const Walk := preload("res://walk.gd")

## Grass is only grown for views from near the ground; from higher up it would not be seen.
const GRASS_EYE_BELOW_M := 30.0
## Frames to let a changed view settle before reading the picture back.
const SETTLE_FRAMES := 6
const MAP_WARMUP_FRAMES := 40
const BENCH_WARMUP_FRAMES := 30
const BENCH_FRAMES := 60
## The walk test knows the station world's rooms: these come from its source and must follow it.
const WALKTEST_WORLD := "Station"
const GROUND_FLOOR_Z := 0.30
const FIRST_FLOOR_Z := 3.55
const KITCHEN_NORTH_WALL_Y := 34.5
const TOWER_AXIS := Vector2(20.0, 32.0)
const WALL_TOLERANCE_M := 0.03
const FLOOR_TOLERANCE_M := 0.05

var tree: SceneTree
var world: WorldData
var marks: Bookmarks
var grass: Grass
var out_dir := ""


func _init(scene_tree: SceneTree, world_data: WorldData, bookmarks: Bookmarks, grass_kit: Grass, out: String) -> void:
	tree = scene_tree
	world = world_data
	marks = bookmarks
	grass = grass_kit
	out_dir = out


func _frames(count: int) -> void:
	for f in count:
		await RenderingServer.frame_post_draw


func _view(b: Dictionary) -> void:
	marks.aim(b)
	if marks.eye_above_ground(b) < GRASS_EYE_BELOW_M:
		grass.grow_around(b["eye"][0], b["eye"][1], Grass.ALL_TILES)


func shoot(picture: SubViewport, only: PackedStringArray) -> void:
	## One PNG per bookmark, named by its id.
	DirAccess.make_dir_recursive_absolute(out_dir)
	for b in marks.list:
		if not only.is_empty() and not only.has(b["id"]):
			continue
		_view(b)
		await _frames(SETTLE_FRAMES)
		picture.get_texture().get_image().save_png("%s/%s.png" % [out_dir, b["id"]])
		print("SHOT ", b["id"])


func bench(window: Window) -> void:
	## Frame rate at each bookmark with vsync off, in the same window the walk mode uses.
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	var total := 0.0
	var count := 0
	for b in marks.list:
		_view(b)
		await _frames(BENCH_WARMUP_FRAMES)
		var t0 := Time.get_ticks_usec()
		await _frames(BENCH_FRAMES)
		var fps := BENCH_FRAMES / (float(Time.get_ticks_usec() - t0) / 1e6)
		total += fps
		count += 1
		print("BENCH %-16s %6.1f fps  %d px wide" % [b["id"], fps, window.size.x])
		if out_dir != "":
			DirAccess.make_dir_recursive_absolute(out_dir)
			window.get_texture().get_image().save_png("%s/walk_%s.png" % [out_dir, b["id"]])
	print("BENCH %-16s %6.1f fps" % ["mean", total / maxf(count, 1)])


func mapshot(window: Window, walk: Walk) -> void:
	## Capture the walking view with the minimap, then with the full map.
	await _frames(MAP_WARMUP_FRAMES)
	DirAccess.make_dir_recursive_absolute(out_dir)
	window.get_texture().get_image().save_png(out_dir + "/runtime_minimap.png")
	walk.show_full_map(true)
	await _frames(SETTLE_FRAMES)
	window.get_texture().get_image().save_png(out_dir + "/runtime_fullmap.png")


func walktest(walk: Walk) -> bool:
	## Scripted checks of the walking body in the station world: the mouse turns the camera, a wall
	## stops the body one body radius short of its face, the floor holds it at the floor's height,
	## and a stair carries it up a level. Prints one line per check; true when all pass.
	var passed := true
	var turn_px := 120.0      # a synthetic mouse move must turn the camera
	var yaw_before := walk.yaw
	var ev := InputEventMouseMotion.new()
	ev.relative = Vector2(turn_px, 0)
	Input.parse_input_event(ev)
	for f in 3:
		await tree.process_frame
	var turned := walk.yaw - yaw_before
	passed = _check("look", Input.mouse_mode == Input.MOUSE_MODE_CAPTURED
			and absf(turned + turn_px * Walk.MOUSE_RAD_PER_PX) < 0.001 and absf(walk.cam.rotation.y - walk.yaw) < 0.001,
		"mouse captured = %s, yaw moved by %.3f rad (want %.3f), camera yaw %.3f (yaw %.3f)" % [
			Input.mouse_mode == Input.MOUSE_MODE_CAPTURED, turned, -turn_px * Walk.MOUSE_RAD_PER_PX, walk.cam.rotation.y, walk.yaw]) and passed
	walk.scripted = true
	walk.fly = false
	var body := walk.body
	# The kitchen is x 30 to 37, y 28.5 to 34.5 on the ground level. Along x = 31.5 nothing solid
	# stands between the middle of the room and the north wall, and the wall there has no door.
	walk.place(WorldData.g3(31.5, 31.0, GROUND_FLOOR_Z + 0.1))
	for f in 150:      # push north into the wall for longer than the run takes
		await tree.physics_frame
		walk.step_scripted(Vector3(0, 0, -Walk.WALK_SPEED))
	var stop_y := -body.global_position.z
	var want_y := KITCHEN_NORTH_WALL_Y - Walk.BODY_RADIUS_M
	passed = _check("wall", absf(stop_y - want_y) < WALL_TOLERANCE_M,
		"stopped at y = %.2f (want %.2f: wall face at %.2f less the body radius)" % [stop_y, want_y, KITCHEN_NORTH_WALL_Y]) and passed
	passed = _check("floor", absf(body.global_position.y - GROUND_FLOOR_Z) < FLOOR_TOLERANCE_M,
		"standing at z = %.2f (floor at %.2f)" % [body.global_position.y, GROUND_FLOOR_Z]) and passed
	var stair_r := 2.75       # the first spiral stair winds about the tower's axis at this radius
	var stair_speed := 3.0
	var a := deg_to_rad(97.0)
	walk.place(WorldData.g3(TOWER_AXIS.x + stair_r * cos(a), TOWER_AXIS.y + stair_r * sin(a), GROUND_FLOOR_Z + 0.05))
	var arrive_z := FIRST_FLOOR_Z - FLOOR_TOLERANCE_M
	for f in 420:      # walk the stair, steering along its arc
		await tree.physics_frame
		var px := body.global_position.x - TOWER_AXIS.x
		var py := -body.global_position.z - TOWER_AXIS.y
		var ang := atan2(py, px)
		var r := Vector2(px, py).length()
		# along the arc, plus a pull back onto it
		var tangent := Vector3(-sin(ang), 0, -cos(ang)) * stair_speed + Vector3(cos(ang), 0, -sin(ang)) * (stair_r - r) * stair_speed
		walk.step_scripted(tangent)
		if body.global_position.y > arrive_z:
			break
	passed = _check("stair", body.global_position.y > arrive_z,
		"reached z = %.2f (first floor at %.2f)" % [body.global_position.y, FIRST_FLOOR_Z]) and passed
	walk.scripted = false
	return passed


func _check(what: String, ok: bool, detail: String) -> bool:
	print("WALKTEST %s %s: %s" % ["PASS" if ok else "FAIL", what, detail])
	return ok
