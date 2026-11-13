extends SceneTree
## Water in the frame at `at_ford` (spec/build.md, the Godot loader's conformance row): the fixture is
## loaded with the water option, every material is replaced by an unshaded flat colour (water
## magenta, all else black, no sky), and the at_ford view is rendered at 1280 by 720. Pixels of the
## water material are then counted exactly, with no renderer's shading in the way. Needs a display:
##   godot --path engines/godot --rendering-method gl_compatibility --script res://tests/test_water.gd
## Exits 0 when at least MIN_SHARE of the frame is water, and the same view without the option has none.

const Loader := preload("res://addons/via_loader/loader.gd")
const MIN_SHARE := 0.02
const WATER := Color(1, 0, 1)


func _initialize() -> void:
	_run.call_deferred()


func _flat(c: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.albedo_color = c
	return m


func _share(water_option: bool) -> float:
	var vp := SubViewport.new()
	vp.size = Vector2i(1280, 720)
	vp.own_world_3d = true
	vp.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color.BLACK
	var we := WorldEnvironment.new()
	we.environment = env
	vp.add_child(we)
	root.add_child(vp)
	var loader := Loader.new()
	var world := loader.make(ProjectSettings.globalize_path("res://tests/fixture_build"),
		{"water": "tag:water_level"} if water_option else {})
	vp.add_child(world)
	var black := _flat(Color.BLACK)
	var magenta := _flat(WATER)
	for n in world.find_children("*", "GeometryInstance3D", true, false):
		n.material_override = magenta if n.name == "water" else black
	(world.get_node("bookmarks/at_ford") as Camera3D).make_current()
	for f in 4:
		await RenderingServer.frame_post_draw
	var img := vp.get_texture().get_image()
	var hits := 0
	for y in img.get_height():
		for x in img.get_width():
			var c := img.get_pixel(x, y)
			if c.r > 0.9 and c.g < 0.1 and c.b > 0.9:
				hits += 1
	vp.queue_free()
	return float(hits) / (img.get_width() * img.get_height())


func _run() -> void:
	var with_water: float = await _share(true)
	var without: float = await _share(false)
	var ok := with_water >= MIN_SHARE and without == 0.0
	print("%s water at at_ford: %.1f%% of the frame with the water option, %.1f%% without" % [
		"ok  " if ok else "FAIL", with_water * 100.0, without * 100.0])
	quit(0 if ok else 1)
