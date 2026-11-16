extends SceneTree
## The walker (walk.gd) and the hand-edit marks (marks.gd) against the hand-made fixture build:
##   godot --path engines/godot --rendering-method gl_compatibility --script res://tests/test_walk.gd
## Prints one line per check and exits 0 when all pass, 1 otherwise.
##
## The fixture's cottage collides as a box, its trees as hulls, and its crate is solid with no
## collision (tests/make_fixture.py), so one build gives every outcome but a fall.

const Loader := preload("res://addons/via_loader/loader.gd")
const Walk := preload("res://walk.gd")
const Marks := preload("res://marks.gd")

const EXPECTED := 25
var failed := 0
var ran := 0


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	ran += 1
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


func _eye(loader, x: float, y: float) -> Vector3:
	## A bookmark's eye over the ground at (x, y): the walk starts from the eye (walk.gd).
	return Vector3(x, y, loader.ground_at(x, y) + Walk.EYE)


func _run() -> void:
	var dir := ProjectSettings.globalize_path("res://tests/fixture_build")
	var loader := Loader.new()
	var root := loader.make(dir, {})
	get_root().add_child(root)
	var unbodied: Array = loader.solid_without_body()
	var names := unbodied.map(func(u): return u.name)
	check("solid instances with no collision are listed, bodied ones are not",
		"crate" in names and "shed" in names and not "cottage" in names and not "trees#t3" in names, str(names))

	check("collision: the fact wins over params", Loader.collision_of({"collision": "hull", "params": {"collision": "box"}}) == "hull"
		and Loader.collision_of({"collision": "none", "params": {"collision": "box"}}) == null)
	check("collision: params when the record has no fact (an M1 build)", Loader.collision_of({"params": {"collision": "mesh"}}) == "mesh"
		and Loader.collision_of({"params": {}}) == null)

	# to_door: from the bookmark to the cottage, which collides as a box: it arrives at the wall
	var r: Dictionary = await Walk.walk(root, loader, _eye(loader, 39.252, 38.736), Vector3(30, 52, 0), "cottage", unbodied)
	check("to_door arrives at the cottage's wall", r.outcome == "arrived" and r.by == "cottage", Walk.summary(r))
	check("to_door stops outside the house", r.left > Walk.ARRIVE, "%.2f m short of its centre" % r.left)
	check("to_door's trail starts at the bookmark and ends where it stopped",
		r.trail.size() >= 2 and Walk.plan(r.trail[0]).distance_to(Vector2(39.252, 38.736)) < 0.01
		and Walk.plan(r.trail[-1]).distance_to(Vector2(r.stop[0], r.stop[1])) < 0.01, str(r.trail.size()))
	check("the walk stood on the ground", absf(r.from[2] - loader.ground_at(39.252, 38.736)) < 1e-4, str(r.from))

	# into a tree that is not the target: blocked, by that tree
	r = await Walk.walk(root, loader, _eye(loader, 36, 4), Vector3(36, 24, 0), "elsewhere", unbodied)
	check("a walk into trees#t3 is blocked by it", r.outcome == "blocked" and r.by == "trees#t3", Walk.summary(r))
	check("blocked short of the tree's trunk", r.stop[1] < 14.0 and r.stop[1] > 9.0, str(r.stop))
	check("blocked has no slope when an instance stops it", r.slope == null, str(r.slope))

	# through the crate, which is solid and has no body
	r = await Walk.walk(root, loader, _eye(loader, 44, 25), Vector3(44, 35, 0), "elsewhere", unbodied)
	check("a walk across the crate arrives", r.outcome == "arrived", Walk.summary(r))
	check("and names the crate it walked through", r.through == ["crate"], str(r.through))
	check("the walked length is about the straight line", absf(r.walked - 9.0) < 0.5, "%.2f" % r.walked)

	# off the world's east edge, where the ground ends: it falls
	r = await Walk.walk(root, loader, _eye(loader, 93, 40), Vector3(110, 40, 0), "elsewhere", unbodied)
	check("a walk off the extent's edge falls", r.outcome == "fell" and r.drop > Walk.FALL and r.stop[0] > 96.0, Walk.summary(r))

	# water deeper than a person wades: a band 1.5 m deep from x = 60 to x = 78, dry beyond
	var wet: Array = []
	for k in loader._height.size():
		var x: float = loader._x0 + (k % loader._nx) * loader._res
		wet.append(loader._height[k] + (1.5 if x >= 60.0 and x <= 78.0 else -1.0))
	var dry_water: PackedFloat32Array = loader._water
	loader._water = PackedFloat32Array(wet)
	r = await Walk.walk(root, loader, _eye(loader, 50, 40), Vector3(85, 40, 0), "elsewhere", unbodied)
	check("a walk into deep water stops at its shore, blocked by water", r.outcome == "blocked" and r.by == "water"
		and r.stop[0] < 60.5 and r.stop[0] > 57.0, Walk.summary(r) + " " + str(r.stop))
	check("it waded nothing deeper than it may", r.deepest == null or r.deepest.depth <= Walk.WADE, str(r.deepest))
	r = await Walk.walk(root, loader, _eye(loader, 50, 40), Vector3(75, 40, 0), "tarn", unbodied)
	check("a walk to a place in deep water arrives at its shore", r.outcome == "arrived" and r.by == "water"
		and r.stop[0] < 60.5, Walk.summary(r) + " " + str(r.stop))
	var under := {"trail": PackedVector3Array([Walk.g(70, 40, loader.ground_at(70, 40))]), "facing": Vector2(1, 0)}
	var cam: Camera3D = Walk.eye(under, [0, 0, 1000, 1000], loader)
	var surface: float = loader.ground_at(70, 40) + loader.depth_at(70, 40)
	check("the eye stands above the water surface", cam.position.y >= surface + Walk.ABOVE_WATER - 1e-4,
		"%.2f over %.2f" % [cam.position.y, surface])
	cam.free()
	check("water is drawn from both sides", loader.water_material().cull_mode == BaseMaterial3D.CULL_DISABLED)
	loader._water = dry_water

	# the walker is removed after each walk
	check("no walker is left in the tree", root.get_node_or_null("walker") == null)

	# the summary line
	check("summary of nothing is `absent`", Walk.summary(null) == "absent")

	# marks from a change report
	var out := ProjectSettings.globalize_path("res://tests/out/marks")
	DirAccess.make_dir_recursive_absolute(out)
	var f := FileAccess.open(out + "/changes.json", FileAccess.WRITE)
	f.store_string(JSON.stringify({"against": {"id": "a", "key": "k"}, "format": "via-changes-0", "summary": [], "entries": [
		{"table": "instance", "name": "oak", "what": ["added"], "before": null, "after": {"asset": "tree_oak", "at": [20, 30, 21], "face": 0}, "causes": [{"cause": "pin.oak"}]},
		{"table": "instance", "name": "trees#t1", "what": ["removed"], "before": {"asset": "tree_oak", "at": [14, 22, 21], "face": 0}, "after": null, "causes": [{"cause": "clear.gap"}]},
		{"table": "instance", "name": "trees#t2", "what": ["removed"], "before": {"asset": "tree_oak", "at": [22, 8, 21], "face": 0}, "after": null, "causes": [{"cause": "pin.oak"}]},
		{"table": "instance", "name": "trees#t9", "what": ["moved"], "before": {"asset": "tree_oak", "at": [1, 1, 1], "face": 0}, "after": {"asset": "tree_oak", "at": [2, 2, 2], "face": 0}, "causes": [{"cause": "scatter.trees.spacing"}]},
	]}))
	f.close()
	var e := Marks.read(out)
	check("a pin's added instance is a pin", e.pins.size() == 1 and e.pins[0].name == "oak" and e.pins[0].cause == "pin.oak", str(e.pins))
	check("cleared, and given way to a pin, are both removed", e.cleared.map(func(c): return c.cause) == ["clear.gap", "pin.oak"], str(e.cleared))
	var m := Marks.node(e, loader)
	check("one pole and ball and ring per pin, one ring and post per removed", m.get_child_count() == 3 + 2 * 2, str(m.get_child_count()))
	m.free()
	root.queue_free()
	check("every check ran", ran >= EXPECTED - 1, "%d of %d" % [ran + 1, EXPECTED])
	print("%s: %d check(s) failed" % ["FAIL" if failed else "PASS", failed])
	quit(1 if failed else 0)
