extends SceneTree
## The loader and walker on more than one surface (build.md 6.5, 10, 15.1), against the synthetic
## build tests/make_floor_fixture.py writes:
##   python3 engines/godot/tests/make_floor_fixture.py DIR
##   godot --headless --path engines/godot --script res://tests/test_floor.gd -- --build DIR
## Headless: it reads only data and physics, never a render. Prints one line per check and exits 0
## when all pass, 1 otherwise.

const Loader := preload("res://addons/via_loader/loader.gd")
const Walk := preload("res://walk.gd")

const EXPECTED := 26
var failed := 0
var ran := 0


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	ran += 1
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var at := args.find("--build")
	if at < 0 or at + 1 >= args.size():
		printerr("test_floor: give -- --build DIR (a build make_floor_fixture.py wrote)")
		quit(2)
		return
	var loader := Loader.new()
	var root := loader.make(args[at + 1], {})
	check("the build loads", root != null, loader.option_error)
	if root == null:
		quit(1)
		return
	get_root().add_child(root)
	check("no load diagnostic but fields_not_drawn", loader.diags.records.all(func(d): return d.code == "load.fields_not_drawn"),
		str(loader.diags.records.map(func(d): return d.code)))

	# the cut: 96 of the 40 by 24 quads, out of the ground mesh and its collision
	check("cut quads are read", loader.is_cut(12, 8) and loader.is_cut(23, 15) and not loader.is_cut(11, 8)
		and not loader.is_cut(24, 12) and not loader.is_cut(12, 16))
	var chunk := root.get_node("c_0_0")
	var tris: int = (chunk.get_node("ground") as MeshInstance3D).mesh.get_meta("triangles")
	check("the ground mesh leaves out the cut quads", tris == 2 * (40 * 24 - 96), "%d triangles" % tris)
	var shape = (chunk.get_node("ground_body/shape") as CollisionShape3D).shape
	check("the ground's collision is a concave shape of the uncut quads", shape is ConcavePolygonShape3D
		and shape.get_faces().size() == 3 * 2 * (40 * 24 - 96), str(shape))

	# the meshes: one node each, its material from the pack's, a body when it collides
	var meshes := root.get_node_or_null("meshes")
	check("a meshes node with the four meshes", meshes != null and meshes.get_child_count() == 4,
		str(meshes.get_children().map(func(n): return n.name)) if meshes != null else "none")
	var hall := meshes.get_node("hall-floor") as MeshInstance3D
	check("hall.floor stands at its south-west corner, 3 m up", hall.position.is_equal_approx(Vector3(12, 3, -8)), str(hall.position))
	check("hall.floor holds its 96 quads' triangles", hall.get_meta("faces") == 192 and hall.get_meta("triangles") == 192, str(hall.get_meta("faces")))
	var colour: Color = hall.material_override.get_shader_parameter("colour")
	check("hall.floor is drawn in its pack material `boards`", colour.is_equal_approx(Color(0.55, 0.36, 0.2)), str(colour))
	var plain: Color = (meshes.get_node("step-floor") as MeshInstance3D).material_override.get_shader_parameter("colour")
	check("a mesh with no material takes the loader's default", plain.is_equal_approx(Loader.DEFAULT_GROUND.srgb_to_linear()), str(plain))
	var body := hall.get_node_or_null("hall-floor") as StaticBody3D
	check("hall.floor collides, its body named for the walker", body != null and body.get_meta("surface") == "hall.floor")

	await physics_frame
	await physics_frame
	var down := Walk.under(root, Walk.g(18, 12, 10), 20.0)
	check("from above, the first thing under the hall is its floor at 3 m", down.get("on") == "hall.floor" and absf(down.y - 3.0) < 1e-4, str(down))
	var hole := Walk.under(root, Walk.g(18, 12, 2.9), 20.0)
	check("under the floor, the cut terrain is not there", hole.is_empty(), str(hole))

	# on the floor: the walker stands at 3 m over the cut and stays on it
	var r: Dictionary = await Walk.walk(root, loader, Vector3(18, 12, 3.0 + Walk.EYE), Vector3(22, 12, 3.0), "hall_east", [])
	check("on_floor stands on hall.floor at 3 m", r.on.from == "hall.floor" and absf(r.from[2] - 3.0) < 1e-3, str(r.on) + " " + str(r.from))
	check("and walks across it to its target, still at 3 m over the cut", r.outcome == "arrived" and r.on.stop == "hall.floor"
		and absf(r.stop[2] - 3.0) < 0.01 and loader.cut_at(r.stop[0], r.stop[1]), Walk.summary(r) + " " + str(r.stop))

	# below the floor, in the cut: nothing under the eye, it falls through the hole
	r = await Walk.walk(root, loader, Vector3(16, 10, 0.0 + Walk.EYE), Vector3(16, 14, 0.0), "hall_north", [])
	check("below_floor stands on nothing and falls through the cut", r.on.from == null and r.outcome == "fell", Walk.summary(r))

	# up the stair tread from the ground onto the floor (20.6 degrees, under MAX_SLOPE)
	r = await Walk.walk(root, loader, Vector3(1, 12, Walk.EYE), Vector3(17, 12, 3.0), "hall", [])
	check("up the tread from the ground onto hall.floor", r.on.from == "ground" and r.outcome == "arrived" and r.on.stop == "hall.floor"
		and absf(r.stop[2] - 3.0) < 0.01, Walk.summary(r) + " " + str(r.stop))

	# off the floor's east edge, where no meeting is: a 3 m ledge
	r = await Walk.walk(root, loader, Vector3(20, 12, 3.0 + Walk.EYE), Vector3(27, 12, 0.0), "elsewhere", [])
	check("off the floor's edge it falls", r.outcome == "fell" and absf(r.drop - 3.0) < 0.1, Walk.summary(r))

	# a step 0.25 m high (under STEP) is climbed; a plinth 0.6 m high (over it) blocks
	r = await Walk.walk(root, loader, Vector3(31, 5, Walk.EYE), Vector3(31, 13, 0.25), "step", [])
	check("onto step.floor, 0.25 m up, within the step height", r.outcome == "arrived" and r.on.stop == "step.floor"
		and absf(r.stop[2] - 0.25) < 0.01, Walk.summary(r) + " " + str(r.stop))
	r = await Walk.walk(root, loader, Vector3(37, 5, Walk.EYE), Vector3(37, 13, 0.6), "elsewhere", [])
	check("plinth.floor, 0.6 m up, blocks", r.outcome == "blocked" and r.by == "plinth.floor" and r.stop[1] < 10.0,
		Walk.summary(r) + " " + str(r.stop))

	# deep water under the hall and west of it (2 m deep from x = 10 to x = 26): the walker on the
	# floor 1 m above its surface walks on; on the ground it stops at the shore (depth against the
	# feet, walk.gd)
	var wet: Array = []
	for k in loader._height.size():
		var x: float = loader._x0 + (k % loader._nx) * loader._res
		wet.append(loader._height[k] + (2.0 if x >= 10.0 and x <= 26.0 else -1.0))
	var dry_water: PackedFloat32Array = loader._water
	loader._water = PackedFloat32Array(wet)
	r = await Walk.walk(root, loader, Vector3(14, 12, 3.0 + Walk.EYE), Vector3(22, 12, 3.0), "hall_east", [])
	check("on hall.floor over 2 m of water it walks on, wading nothing", r.outcome == "arrived" and r.on.stop == "hall.floor"
		and r.deepest == null, Walk.summary(r) + " " + str(r.deepest))
	r = await Walk.walk(root, loader, Vector3(1, 12, Walk.EYE), Vector3(17, 12, 3.0), "hall", [])
	check("up the tread over the water onto hall.floor", r.outcome == "arrived" and r.on.stop == "hall.floor"
		and r.deepest == null, Walk.summary(r) + " " + str(r.deepest))
	r = await Walk.walk(root, loader, Vector3(5, 3, Walk.EYE), Vector3(32, 3, 0.0), "elsewhere", [])
	check("on the ground it stops at the shore, blocked by water", r.outcome == "blocked" and r.by == "water"
		and r.stop[0] > 8.5 and r.stop[0] < 10.0, Walk.summary(r) + " " + str(r.stop))
	# off the floor's east edge toward a point in the water: it falls, though the target is wet
	# (city's bridges walked off their decks and said `arrived`)
	r = await Walk.walk(root, loader, Vector3(20, 12, 3.0 + Walk.EYE), Vector3(25.8, 12, 0.0), "in_water", [])
	check("off the floor's edge into deep water it falls, not arrives", r.outcome == "fell" and r.drop > Walk.FALL,
		Walk.summary(r) + " " + str(r.stop))
	loader._water = dry_water

	# along a path (a bookmark's `along`): north 3 m, east 5 m, then to the target 3 m south; a
	# straight walk would be 5 m
	r = await Walk.walk(root, loader, Vector3(1, 3, Walk.EYE), Vector3(6, 3, 0.0), "elsewhere", [], [[1, 6], [6, 6]])
	var most_north := -INF
	for q in r.trail:
		most_north = maxf(most_north, Walk.plan(q).y)
	check("along a path it passes each point, then arrives", r.outcome == "arrived" and most_north > 5.6
		and r.walked > 8.5 and r.walked < 11.0, Walk.summary(r) + " north %.2f" % most_north)

	root.queue_free()
	check("every check ran", ran >= EXPECTED - 1, "%d of %d" % [ran + 1, EXPECTED])
	print("%s: %d check(s) failed" % ["FAIL" if failed else "PASS", failed])
	quit(1 if failed else 0)
