extends Node
## The showcase: one build drawn to be looked at, not measured (docs/design/demo.md). The sheet's
## rig (rig.gd) is an instrument: a linear tone map, a fixed neutral ambient, the sky not
## reflected, the Compatibility renderer, so that a colour drawn is the colour declared. This one
## is a picture: Forward+, a low warm sun with soft shadows, the sky's light and reflections, a
## filmic tone map, ambient occlusion and a little aerial fog. The build is loaded through the same
## loader (addons/via_loader), so what is drawn is the build; nothing is added to it.
##
##   godot --path engines/godot --rendering-method forward_plus res://showcase.tscn -- \
##       --build DIR --out FILE.png [--pack DIR] [--bookmark NAME | --eye X,Y,Z --at X,Y,Z]
##       [--size W,H] [--fov DEGREES] [--sun BEARING,ELEVATION] [--option NAME=VALUE ...]
##
## Coordinates are the world's (x east, y north, z up, metres). Exit 0 when the picture is written.

const Loader := preload("res://addons/via_loader/loader.gd")
const Rig := preload("res://rig.gd")

## Frames drawn before the picture is taken: shadows, the occlusion and the exposure settle.
const SETTLE_FRAMES := 24
const PARKED := Vector2i(160, 90)

var build := ""
var out := ""
var pack := ""
var bookmark := ""
var eye = null
var at = null
var size := Vector2i(1920, 1080)
var fov := 55.0
var sun_bearing := 250.0
var sun_elevation := 24.0
var options := {}


func _ready() -> void:
	var err := _read_arguments()
	if err != "":
		printerr("showcase: " + err)
		get_tree().quit(2)
		return
	_park_window()
	_run.call_deferred()


func _abs(path: String) -> String:
	if path == "" or path.is_absolute_path():
		return path.simplify_path()
	var cwd := OS.get_environment("PWD")
	return (cwd + "/" + path).simplify_path() if cwd != "" else path


static func _three(v: String):
	var p := v.split(",")
	if p.size() != 3 or not (p[0].is_valid_float() and p[1].is_valid_float() and p[2].is_valid_float()):
		return null
	return Vector3(float(p[0]), float(p[1]), float(p[2]))


func _read_arguments() -> String:
	var args := OS.get_cmdline_user_args()
	var n := 0
	while n < args.size():
		var a := args[n]
		if not a in ["--build", "--out", "--pack", "--bookmark", "--eye", "--at", "--size", "--fov", "--sun", "--option"]:
			return "unknown argument `%s`" % a
		if n + 1 >= args.size():
			return "`%s` takes a value" % a
		var v := args[n + 1]
		match a:
			"--build": build = _abs(v)
			"--out": out = _abs(v)
			"--pack": pack = _abs(v)
			"--bookmark": bookmark = v
			"--eye", "--at":
				var p = _three(v)
				if p == null:
					return "%s takes X,Y,Z, not `%s`" % [a, v]
				if a == "--eye":
					eye = p
				else:
					at = p
			"--size":
				var p := v.split(",")
				if p.size() != 2 or not (p[0].is_valid_int() and p[1].is_valid_int()):
					return "--size takes W,H, not `%s`" % v
				size = Vector2i(int(p[0]), int(p[1]))
			"--fov":
				if not v.is_valid_float():
					return "--fov takes degrees, not `%s`" % v
				fov = float(v)
			"--sun":
				var p := v.split(",")
				if p.size() != 2 or not (p[0].is_valid_float() and p[1].is_valid_float()):
					return "--sun takes BEARING,ELEVATION, not `%s`" % v
				sun_bearing = float(p[0])
				sun_elevation = float(p[1])
			"--option":
				var eq := v.find("=")
				if eq <= 0:
					return "--option takes NAME=VALUE, not `%s`" % v
				var e := Loader.parse_option(options, v.substr(0, eq), v.substr(eq + 1))
				if e != "":
					return "--option " + e
		n += 2
	if build == "" or out == "":
		return "--build and --out are required"
	if (eye == null) != (at == null):
		return "--eye and --at go together"
	if bookmark == "" and eye == null:
		return "name a --bookmark, or give --eye and --at"
	return ""


func _park_window() -> void:
	var win := get_window()
	win.unfocusable = true
	win.borderless = true
	win.size = PARKED
	var screen := DisplayServer.window_get_current_screen()
	win.position = DisplayServer.screen_get_position(screen) + DisplayServer.screen_get_size(screen) - PARKED - Vector2i(8, 8)


func _to_sun() -> Vector3:
	var b := deg_to_rad(sun_bearing)
	var e := deg_to_rad(sun_elevation)
	return Rig.g(sin(b) * cos(e), cos(b) * cos(e), sin(e))


func _rig(vp: SubViewport) -> void:
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = Sky.new()
	var sky := ProceduralSkyMaterial.new()
	sky.sky_top_color = Color(0.22, 0.42, 0.78)
	sky.sky_horizon_color = Color(0.78, 0.82, 0.86)
	# Below the horizon is past the world's edge, where nothing is drawn: the haze's colour, so the
	# edge reads as distance.
	sky.ground_horizon_color = Color(0.78, 0.82, 0.86)
	sky.ground_bottom_color = Color(0.78, 0.82, 0.86)
	sky.sun_angle_max = 20.0
	sky.sun_curve = 0.08
	env.sky.sky_material = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 0.45
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.tonemap_exposure = 1.0
	env.tonemap_white = 6.0
	env.ssao_enabled = true
	env.ssao_radius = 1.6
	env.ssao_intensity = 2.2
	env.ssil_enabled = true
	env.glow_enabled = true
	env.glow_intensity = 0.35
	env.glow_bloom = 0.04
	env.fog_enabled = true
	env.fog_light_color = Color(0.78, 0.82, 0.86)
	env.fog_density = 0.00045
	env.fog_aerial_perspective = 0.2
	env.fog_sky_affect = 0.0
	env.adjustment_enabled = true
	env.adjustment_saturation = 1.22
	env.adjustment_contrast = 1.12
	var we := WorldEnvironment.new()
	we.environment = env
	vp.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.light_color = Color(1.0, 0.82, 0.6)
	sun.light_energy = 2.8
	sun.light_angular_distance = 1.2
	sun.transform = Transform3D(Basis.looking_at(-_to_sun(), Vector3.UP), Vector3.ZERO)
	sun.shadow_enabled = true
	sun.shadow_blur = 1.5
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 600.0
	vp.add_child(sun)


func _run() -> void:
	var vp := SubViewport.new()
	vp.size = size
	vp.msaa_3d = Viewport.MSAA_4X
	vp.screen_space_aa = Viewport.SCREEN_SPACE_AA_FXAA
	vp.world_3d = World3D.new()
	vp.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(vp)
	_rig(vp)
	var loader := Loader.new()
	loader.pack_dir = pack
	var root: Node3D = loader.make(build, options)
	loader.diags.print_all()
	if root == null:
		printerr("showcase: the build did not load")
		get_tree().quit(1)
		return
	vp.add_child(root)
	var cam := Camera3D.new()
	vp.add_child(cam)
	if bookmark != "":
		var found: Camera3D = null
		for c in root.get_node("bookmarks").get_children():
			if String(c.get_meta("feature")) == bookmark or String(c.name) == bookmark:
				found = c
		if found == null:
			printerr("showcase: no bookmark `%s` in the build" % bookmark)
			get_tree().quit(2)
			return
		cam.global_transform = found.global_transform
	else:
		var e := Rig.g(eye.x, eye.y, eye.z)
		cam.look_at_from_position(e, Rig.g(at.x, at.y, at.z), Vector3.UP)
	cam.fov = fov
	cam.near = 0.1
	cam.far = 4000.0
	cam.make_current()
	for f in SETTLE_FRAMES:
		await RenderingServer.frame_post_draw
	var img := vp.get_texture().get_image()
	img.convert(Image.FORMAT_RGB8)
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	var err := img.save_png(out)
	if err != OK:
		printerr("showcase: cannot write %s (%s)" % [out, error_string(err)])
		get_tree().quit(1)
		return
	print("showcase: %s" % out)
	get_tree().quit(0)
