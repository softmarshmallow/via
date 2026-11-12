extends RefCounted
## Placed assets. Turns each asset into drawable parts (a primitive, a model file or a recipe),
## batches the scattered instances per chunk, and gives hand-placed things a box to walk around.

const WorldData := preload("res://world_data.gd")

## An asset's yaw is measured from world east; the engine's zero faces the other axis.
const YAW_OFFSET_DEG := 90.0
## Recipe trees swap their full crown and trunk for the thinner far crown at this distance.
const LOD_SWAP_M := 70.0
## Scattered things stop drawing at a distance that grows with their height, within these bounds.
const REACH_PER_M := 55.0
const REACH_MIN_M := 70.0
const REACH_MAX_M := 900.0
## Things shorter than this cast no shadow while walking.
const SHADOW_MIN_HEIGHT_M := 3.0
const CULL_MARGIN_M := 4.0
const ALPHA_SCISSOR := 0.5
const BARK := Color(0.30, 0.22, 0.15)
const RECIPE_BARK := [0.40, 0.33, 0.28]

var world: WorldData
var stage: Node3D
var shots := false
var white_tex: ImageTexture
var _recipe_scripts: Dictionary = {}


func _init(world_data: WorldData, stage_node: Node3D, for_shots: bool) -> void:
	world = world_data
	stage = stage_node
	shots = for_shots
	var img := Image.create(1, 1, false, Image.FORMAT_RGB8)
	img.fill(Color.WHITE)
	white_tex = ImageTexture.create_from_image(img)


func build() -> void:
	## One batch per asset part per chunk, so the engine can cull it. Small things stop drawing and
	## stop casting shadows sooner than large ones.
	var assets: Dictionary = world.doc["assets"]
	var chunk := world.chunk
	var batches := 0
	for aid in world.doc["instances"]:
		var a: Dictionary = assets[aid]
		var tall := float(a["size"][2])
		var reach := 0.0 if shots else clampf(tall * REACH_PER_M, REACH_MIN_M, REACH_MAX_M)
		var casts := shots or tall >= SHADOW_MIN_HEIGHT_M
		var parts := _asset_parts(a)
		var variants := 1
		for part in parts:
			if part.size() > 2:
				variants = maxi(variants, int(part[2]) + 1)
		var cells: Dictionary = world.doc["instances"][aid]
		for cell in cells:
			var rows: Array = cells[cell]
			var cx := int(String(cell).split(",")[0])
			var cy := int(String(cell).split(",")[1])
			var x_mid := world.x_min + (cx + 0.5) * chunk
			var y_mid := world.y_min + (cy + 0.5) * chunk
			var origin := WorldData.g3(x_mid, y_mid, world.h_at(x_mid, y_mid))
			for part in parts:
				var mine: Array = []
				for k in rows.size():
					if part.size() < 3 or k % variants == int(part[2]):
						mine.append(rows[k])
				if mine.is_empty():
					continue
				var mmi := MultiMeshInstance3D.new()
				mmi.multimesh = _batch(part, mine, origin)
				mmi.position = origin
				mmi.extra_cull_margin = CULL_MARGIN_M
				if not casts:
					mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
				var begin := float(part[3]) if part.size() > 3 else 0.0
				var end := float(part[4]) if part.size() > 4 else 0.0
				if begin > 0.0:
					mmi.visibility_range_begin = begin
				if end > 0.0:
					mmi.visibility_range_end = end
				elif reach > 0.0:
					mmi.visibility_range_end = reach
				stage.add_child(mmi)
				batches += 1
	print("BATCHES ", batches)


func _batch(part: Array, rows: Array, origin: Vector3) -> MultiMesh:
	## Each row is [x, y, z, yaw, scale, tint r, g, b]. Transforms are kept relative to the chunk.
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = part[0]
	mm.instance_count = rows.size()
	for k in rows.size():
		var r: Array = rows[k]
		var basis := Basis(Vector3.UP, deg_to_rad(float(r[3]) + YAW_OFFSET_DEG)).scaled(Vector3.ONE * float(r[4]))
		var xf := Transform3D(basis, WorldData.g3(float(r[0]), float(r[1]), float(r[2])) - origin) * (part[1] as Transform3D)
		mm.set_instance_transform(k, xf)
		mm.set_instance_custom_data(k, Color(float(r[5]), float(r[6]), float(r[7]), 1.0))
	return mm


func add_colliders() -> void:
	## Hand-placed solid things are boxes to walk around.
	var assets: Dictionary = world.doc["assets"]
	for e in world.doc.get("explicit", []):
		var a: Dictionary = assets[e["asset"]]
		if not a.get("solid", true):
			continue
		var tall := float(a["size"][2])
		var box := BoxShape3D.new()
		# the footprint is what is solid on the ground (a trunk), not the model's whole extent (a crown)
		box.size = Vector3(float(a["footprint"][0]), tall, float(a["footprint"][1]))
		var cs := CollisionShape3D.new()
		cs.shape = box
		var sb := StaticBody3D.new()
		sb.add_child(cs)
		sb.position = WorldData.g3(e["pos"][0], e["pos"][1], float(e["pos"][2]) + tall * 0.5)
		sb.rotation.y = deg_to_rad(float(e["yaw"]) + YAW_OFFSET_DEG)
		stage.add_child(sb)


func _asset_parts(a: Dictionary) -> Array:
	## Each part is [mesh, local transform], and for recipes also [variant, drawn from, drawn to].
	if a.has("primitive"):
		return _primitive_parts(a)
	if a.has("recipe"):
		return _recipe_parts(a)
	return _model_parts(a)


func _prop_material(tex: Texture2D, col: Color, scissor: float) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = load("res://prop.gdshader")
	mat.set_shader_parameter("albedo_tex", tex if tex != null else white_tex)
	mat.set_shader_parameter("albedo_col", col)
	mat.set_shader_parameter("scissor", scissor)
	return mat


func _cylinder(bottom_r: float, top_r: float, tall: float, segments: int, col: Color) -> CylinderMesh:
	var m := CylinderMesh.new()
	m.top_radius = top_r
	m.bottom_radius = bottom_r
	m.height = tall
	m.radial_segments = segments
	m.material = _prop_material(null, col, 0.0)
	return m


func _sphere(radius: float, tall: float, segments: int, rings: int, col: Color) -> SphereMesh:
	var m := SphereMesh.new()
	m.radius = radius
	m.height = tall
	m.radial_segments = segments
	m.rings = rings
	m.material = _prop_material(null, col, 0.0)
	return m


static func _raised(y: float) -> Transform3D:
	return Transform3D(Basis(), Vector3(0, y, 0))


func _primitive_parts(a: Dictionary) -> Array:
	## Stand-in shapes. `size` is [across, depth, height]; each shape stands on the ground at its origin.
	var s: Array = a["size"]
	var w := float(s[0])
	var d := float(s[1])
	var h := float(s[2])
	var col := WorldData.rgb(a["color"])
	var parts: Array = []
	match String(a["primitive"]):
		"box":
			var m := BoxMesh.new()
			m.size = Vector3(w, h, d)
			m.material = _prop_material(null, col, 0.0)
			parts.append([m, _raised(h * 0.5)])
		"house", "gable":      # walls under a pitched roof; a house's ridge runs along its front, a gable's from front to back
			var wall_h := h * 0.66
			var walls := BoxMesh.new()
			walls.size = Vector3(w, wall_h, d)
			walls.material = _prop_material(null, col, 0.0)
			parts.append([walls, _raised(wall_h * 0.5)])
			var roof := PrismMesh.new()
			var along_front := String(a["primitive"]) == "house"
			roof.size = Vector3(d * 1.05, h - wall_h, w * 1.0) if along_front else Vector3(w * 1.05, h - wall_h, d * 1.03)
			roof.material = _prop_material(null, WorldData.rgb(a.get("roof", [0.43, 0.25, 0.20])), 0.0)
			var lifted := Vector3(0, wall_h + (h - wall_h) * 0.5, 0)
			parts.append([roof, Transform3D(Basis(Vector3.UP, PI * 0.5), lifted) if along_front else Transform3D(Basis(), lifted)])
		"tower":      # a round tower with a dark lantern and a cap
			parts.append([_cylinder(w * 0.5, w * 0.40, h * 0.78, 20, col), _raised(h * 0.39)])
			parts.append([_cylinder(w * 0.52, w * 0.52, h * 0.03, 20, Color(0.80, 0.62, 0.30)), _raised(h * 0.79)])
			parts.append([_cylinder(w * 0.30, w * 0.30, h * 0.12, 12, Color(0.10, 0.11, 0.12)), _raised(h * 0.86)])
			parts.append([_cylinder(w * 0.34, 0.0, h * 0.08, 12, Color(0.08, 0.08, 0.09)), _raised(h * 0.96)])
		"cylinder":
			parts.append([_cylinder(w * 0.5, w * 0.5, h, 14, col), _raised(h * 0.5)])
		"cone":       # a conifer: a short trunk under a cone
			parts.append([_cylinder(w * 0.07, w * 0.05, h * 0.3, 8, BARK), _raised(h * 0.15)])
			parts.append([_cylinder(w * 0.5, 0.0, h * 0.84, 10, col), _raised(h * 0.58)])
		"tree":       # a broadleaf: a trunk under a round crown
			parts.append([_cylinder(w * 0.08, w * 0.05, h * 0.5, 8, BARK), _raised(h * 0.25)])
			parts.append([_sphere(w * 0.5, h * 0.64, 12, 7, col), _raised(h * 0.68)])
		_:
			parts.append([_sphere(0.5, 1.0, 10, 6, col), Transform3D(Basis().scaled(Vector3(w, h, d)), Vector3(0, h * 0.42, 0))])
	return parts


func _model_parts(a: Dictionary) -> Array:
	## A model file, flattened to its meshes and fitted by the scale and lift the compiler worked out.
	var doc := GLTFDocument.new()
	var state := GLTFState.new()
	if doc.append_from_file(a["source"], state) != OK:
		push_error("cannot load " + String(a["source"]))
		return []
	var scene := doc.generate_scene(state)
	var tex: Texture2D = null
	if a.has("albedo"):
		tex = _mipmapped(a["albedo"])
	var parts: Array = []
	var fit := Transform3D(Basis().scaled(Vector3.ONE * float(a["scale"])), Vector3(0, float(a["lift"]), 0))
	_collect(scene, fit, tex, parts, WorldData.rgb(a["color"]))
	scene.free()
	return parts


func _mipmapped(path: String) -> ImageTexture:
	var img := Image.load_from_file(path)
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


func _collect(node: Node, xf: Transform3D, override_tex: Texture2D, parts: Array, fallback: Color) -> void:
	## Walk a model's nodes, giving every surface the prop material. A surface with neither a
	## texture nor a colour of its own takes the asset's declared colour.
	var here := xf
	if node is Node3D:
		here = xf * (node as Node3D).transform
	if node is MeshInstance3D:
		var mesh: Mesh = (node as MeshInstance3D).mesh.duplicate()
		for s in mesh.get_surface_count():
			var src := mesh.surface_get_material(s)
			var tex: Texture2D = override_tex
			var col := Color.WHITE
			var scissor := 0.0
			if src is BaseMaterial3D:
				var bm := src as BaseMaterial3D
				if tex == null:
					tex = bm.albedo_texture
					col = bm.albedo_color
				if bm.transparency != BaseMaterial3D.TRANSPARENCY_DISABLED:
					scissor = ALPHA_SCISSOR
			if tex == null and (src == null or (src is BaseMaterial3D and (src as BaseMaterial3D).albedo_texture == null and col == Color.WHITE)):
				col = fallback
			mesh.surface_set_material(s, _prop_material(tex, col, scissor))
		parts.append([mesh, here])
	for child in node.get_children():
		_collect(child, here, override_tex, parts, fallback)


func _recipe_script(path: String) -> GDScript:
	## Recipes are scripts from the pack, loaded as text. Their own class name is stripped so that
	## loading one never collides with a name the project already knows.
	if not _recipe_scripts.has(path):
		var gd := GDScript.new()
		var src := FileAccess.get_file_as_string(path)
		var name_at := src.find("class_name ")
		var own_name := src.substr(name_at + 11, src.find("\n", name_at) - name_at - 11).strip_edges() if name_at >= 0 else ""
		# plain text replacement: it would also rewrite the name inside a string or a longer identifier
		if own_name != "":
			src = src.replace("class_name " + own_name, "").replace(own_name + ".", "")
		gd.source_code = src
		if gd.reload() != OK:
			push_error("recipe script did not load: " + path)
			return null
		_recipe_scripts[path] = gd
	return _recipe_scripts[path]


func _recipe_parts(a: Dictionary) -> Array:
	## An engine-side generator named by the pack. Its meshes are used as built; each variant is one tree.
	var rec: Dictionary = a["recipe"]
	var gd := _recipe_script(String(rec["script"]))
	if gd == null:
		return []
	var atlas: Texture2D = white_tex
	if rec.has("atlas"):
		atlas = _mipmapped(rec["atlas"])
	var leaf := ShaderMaterial.new()
	leaf.shader = load("res://leaf.gdshader")
	leaf.set_shader_parameter("leaf_tex", atlas)
	leaf.set_shader_parameter("tint", WorldData.rgb(rec.get("tint", WorldData.WHITE_RGB)))
	leaf.set_shader_parameter("bark", WorldData.rgb(rec.get("bark", RECIPE_BARK)))
	var wood := ShaderMaterial.new()
	wood.shader = load("res://vcol.gdshader")
	# near: full crown and trunk. Far: the recipe's own thinner crown with its stem baked in.
	# each band is [mesh name, drawn from this distance, to this distance (0 = no limit)]
	var bands := [["canopy", 0.0, 0.0], ["trunk", 0.0, 0.0]]
	if not shots:
		bands = [["canopy", 0.0, LOD_SWAP_M], ["canopy_far", LOD_SWAP_M, 0.0], ["trunk", 0.0, LOD_SWAP_M]]
	var parts: Array = []
	var variants: Array = rec.get("variants", [rec.get("args", [])])
	for v in variants.size():
		var maker: Object = gd.new()
		var call_args: Array = []
		for arg in variants[v]:      # JSON has no integers; a recipe's signature does
			call_args.append(int(arg) if typeof(arg) == TYPE_FLOAT and float(arg) == floor(float(arg)) else arg)
		var built: Dictionary = maker.callv(String(rec["call"]), call_args)
		print("RECIPE ", rec["call"], " ", variants[v], " height ", built.get("height", 0.0), " crown ", built.get("crown_radius", 0.0))
		for band in bands:
			var mesh: ArrayMesh = built.get(band[0])
			if mesh == null or mesh.get_surface_count() == 0:
				continue
			mesh.surface_set_material(0, wood if band[0] == "trunk" else leaf)
			parts.append([mesh, Transform3D.IDENTITY, v, band[1], band[2]])
	return parts
