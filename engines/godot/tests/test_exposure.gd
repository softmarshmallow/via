extends SceneTree
## Exposure under the sheet's rig (rig.gd): known colours, drawn through the loader's instance
## shader as a pack's box is, are rendered and their lit faces read back. Needs a display:
##   godot --path engines/godot --rendering-method gl_compatibility --script res://tests/test_exposure.gd
##
## Each box's south face is vertical, facing bearing 180; the sun at bearing 225 and 45 degrees up
## meets it at N.L = cos 45 * cos 45 = 0.5. Checks: a grey card of albedo 0.5 lands mid-scale; a
## pack's colours (house_low's wall, the tree's green, a near-white) are not clipped and keep their
## declared channel order and ratios; and a near-white turned full on to the sun (N.L 1), the
## brightest a pack colour is lit, is not clipped either. Prints one line per check; exit 0 when all
## pass.

const Primitives := preload("res://addons/via_loader/primitives.gd")
const Rig := preload("res://rig.gd")
const INSTANCE_SHADER := preload("res://addons/via_loader/instance.gdshader")

## Linear albedo, as a pack declares it (assets.md 2.6).
const CARDS := {
	"grey card 0.5": Color(0.5, 0.5, 0.5),
	"house_low wall": Color(0.55, 0.42, 0.33),
	"tree green": Color(0.18, 0.35, 0.12),
	"near white 0.8": Color(0.8, 0.78, 0.75),
	"near white 0.8, full on to the sun": Color(0.8, 0.78, 0.75),
}
## The cards turned to face the sun (N.L 1); the rest face south (N.L 0.5). Last in the row, east,
## so the shadow it casts north-east falls on none of the others.
const TO_SUN := ["near white 0.8, full on to the sun"]
const SPACING := 3.0
## The grey card's lit face, in sRGB 0 to 1: mid-scale.
const MID := [0.4, 0.6]
## No channel of a lit face at or above this (of 255): not clipped.
const CLIP := 250
## Channel ratios (linear, read back over declared) within this share.
const RATIO_TOL := 0.08

var failed := 0


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


func _basis(name: String) -> Basis:
	## The box's south face (Godot +Z) turned to the sun for a TO_SUN card.
	return Basis.looking_at(-Rig.to_sun(), Vector3.UP) if name in TO_SUN else Basis.IDENTITY


func _box(colour: Color, x: float, basis: Basis) -> MultiMeshInstance3D:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = Primitives.mesh("box", [-1, -1, 0, 1, 1, 2])
	mm.instance_count = 1
	mm.set_instance_transform(0, Transform3D(basis, Rig.g(x, 0, 0)))
	mm.set_instance_custom_data(0, Color(1, 0, 0, 0))   # tint 1, as the loader writes it
	var mat := ShaderMaterial.new()
	mat.shader = INSTANCE_SHADER
	mat.set_shader_parameter("colour", Vector3(colour.r, colour.g, colour.b))
	var node := MultiMeshInstance3D.new()
	node.multimesh = mm
	node.material_override = mat
	return node


func _mean(img: Image, at: Vector2) -> Color:
	## The mean of a 9 by 9 patch, in sRGB 0 to 1.
	var acc := Color(0, 0, 0, 0)
	for dy in range(-4, 5):
		for dx in range(-4, 5):
			acc += img.get_pixelv(Vector2i(at) + Vector2i(dx, dy))
	return acc / 81.0


func _run() -> void:
	var vp := SubViewport.new()
	vp.size = Vector2i(1280, 720)
	vp.own_world_3d = true
	vp.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(vp)
	Rig.rig(vp)
	var names := CARDS.keys()
	var x0 := -SPACING * (names.size() - 1) / 2.0
	for k in names.size():
		vp.add_child(_box(CARDS[names[k]], x0 + k * SPACING, _basis(names[k])))
	var cam := Camera3D.new()
	vp.add_child(cam)
	# south of the row, at the faces' mid-height, looking north
	cam.transform = Transform3D(Basis.looking_at(Rig.g(0, 1, 0), Vector3.UP), Rig.g(0, -9, 1))
	cam.make_current()
	for f in 6:
		await RenderingServer.frame_post_draw
	var img := vp.get_texture().get_image()
	img.convert(Image.FORMAT_RGBF)
	check("the south face meets the sun at N.L 0.5", is_equal_approx(Rig.to_sun().dot(Rig.g(0, -1, 0)), 0.5),
		str(Rig.to_sun().dot(Rig.g(0, -1, 0))))
	check("a turned card's face meets the sun at N.L 1", is_equal_approx(Rig.to_sun().dot(_basis(TO_SUN[0]) * Vector3(0, 0, 1)), 1.0))
	for k in names.size():
		var name: String = names[k]
		var want: Color = CARDS[name]
		# the centre of the south face: (0, 1, 1) in the box's own Godot axes, turned with it
		var at := cam.unproject_position(Rig.g(x0 + k * SPACING, 0, 0) + _basis(name) * Vector3(0, 1, 1))
		var s := _mean(img, at)
		var lin := s.srgb_to_linear()
		var px := "sRGB (%d, %d, %d), linear (%.3f, %.3f, %.3f)" % [roundi(s.r * 255), roundi(s.g * 255), roundi(s.b * 255), lin.r, lin.g, lin.b]
		check("%s: not clipped" % name, maxf(s.r, maxf(s.g, s.b)) * 255.0 < CLIP, px)
		var order_want := _order(want)
		check("%s: keeps its channel order" % name, _order(lin) == order_want or _flat(want), px)
		var k_r := lin.r / want.r
		var ok := absf(lin.g / want.g / k_r - 1.0) <= RATIO_TOL and absf(lin.b / want.b / k_r - 1.0) <= RATIO_TOL
		check("%s: keeps its channel ratios within %d%%" % [name, roundi(RATIO_TOL * 100)], ok,
			"%s; scale r %.3f g %.3f b %.3f" % [px, k_r, lin.g / want.g, lin.b / want.b])
		if name == "grey card 0.5":
			check("grey card 0.5 at N.L 0.5 lands mid-scale (sRGB %.2f to %.2f)" % MID, s.r >= MID[0] and s.r <= MID[1], px)
	vp.queue_free()
	print("%s: %d check(s) failed" % ["FAIL" if failed else "PASS", failed])
	quit(1 if failed else 0)


static func _flat(c: Color) -> bool:
	return c.r == c.g or c.g == c.b or c.r == c.b


static func _order(c: Color) -> Array:
	var ch := [["r", c.r], ["g", c.g], ["b", c.b]]
	ch.sort_custom(func(a, b): return a[1] > b[1])
	return ch.map(func(a): return a[0])
