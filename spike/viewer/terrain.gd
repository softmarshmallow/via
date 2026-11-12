extends RefCounted
## The ground and the water over it: the terrain material, the meshes it is drawn on (one sheet for
## shots, distance levels per chunk for walking), the water surface and the open-sea plane.

const WorldData := preload("res://world_data.gd")

const GROUND_TEX_PX := 1024
## How many metres one repeat of a ground texture covers when its material does not say.
const DEFAULT_TILE_M := 4.0
const WATER_GRID_M := 2.0
## The grids are flat until the shader lifts them, so their culling boxes are given by hand.
const SHEET_BOX_LOW_M := -200.0
const SHEET_BOX_TALL_M := 600.0
const CHUNK_BOX_PAD_M := 6.0
## A chunk's height range is found from this many samples along each side.
const CHUNK_PROBES := 9
const OPEN_SEA_SIZE_M := 6000.0
## The open sea sits just under the water field so the two never fight for the same pixels.
const OPEN_SEA_DROP_M := 0.25

var world: WorldData
var stage: Node3D
var shots := false
var material: ShaderMaterial


func _init(world_data: WorldData, stage_node: Node3D, for_shots: bool) -> void:
	world = world_data
	stage = stage_node
	shots = for_shots


func build_material() -> void:
	var images: Array[Image] = []
	var tiles: Array = []
	for g in world.doc["ground"]:
		images.append(_ground_image(g))
		tiles.append(float(g.get("scale_m", DEFAULT_TILE_M)))
	while images.size() < WorldData.GROUND_LAYERS:
		images.append(images[0])
		tiles.append(DEFAULT_TILE_M)
	var layers := Texture2DArray.new()
	layers.create_from_images(images)
	material = ShaderMaterial.new()
	material.shader = load("res://terrain.gdshader")
	material.set_shader_parameter("height_tex", world.field_texture(world.height))
	material.set_shader_parameter("w0", world.weight_texture(world.w0))
	material.set_shader_parameter("w1", world.weight_texture(world.w1))
	material.set_shader_parameter("ground", layers)
	material.set_shader_parameter("extent", world.extent4())
	material.set_shader_parameter("posts", Vector2(world.nx, world.ny))
	material.set_shader_parameter("res", world.res)
	material.set_shader_parameter("tile", PackedFloat32Array(tiles))
	if world.hole.is_empty():
		var none := Image.create(1, 1, false, Image.FORMAT_RF)
		none.fill(Color(0, 0, 0))
		material.set_shader_parameter("hole_tex", ImageTexture.create_from_image(none))
	else:
		material.set_shader_parameter("hole_tex", world.field_texture(world.hole))


func _ground_image(g: Dictionary) -> Image:
	## One ground material as a square picture: its texture, tinted, or else its flat colour.
	var img: Image
	if g.has("texture"):
		img = Image.load_from_file(g["texture"])
		img.convert(Image.FORMAT_RGB8)
		img.resize(GROUND_TEX_PX, GROUND_TEX_PX, Image.INTERPOLATE_LANCZOS)
		var tint: Array = g.get("tint", WorldData.WHITE_RGB)
		if tint != WorldData.WHITE_RGB:
			for py in GROUND_TEX_PX:
				for px in GROUND_TEX_PX:
					var c := img.get_pixel(px, py).srgb_to_linear()
					img.set_pixel(px, py, Color(c.r * tint[0], c.g * tint[1], c.b * tint[2]).linear_to_srgb())
	else:
		img = Image.create(GROUND_TEX_PX, GROUND_TEX_PX, false, Image.FORMAT_RGB8)
		img.fill(WorldData.rgb(g["color"]).linear_to_srgb())
	img.generate_mipmaps()
	return img


func build_ground() -> void:
	if shots:
		_place_sheet(_sheet_mesh(world.res), material, true)
		return
	# walking: one piece per chunk, coarser with distance, so the engine can cull and thin it
	var chunk := world.chunk
	# each level is [grid step, drawn from this distance, to this distance (0 = no limit)]
	var levels := [[world.res, 0.0, 100.0], [world.res * 2.0, 100.0, 240.0], [world.res * 8.0, 240.0, 0.0]]
	var meshes: Array = []
	for lv in levels:
		var m := PlaneMesh.new()
		m.size = Vector2(chunk, chunk)
		m.subdivide_width = int(chunk / float(lv[0])) - 1
		m.subdivide_depth = int(chunk / float(lv[0])) - 1
		meshes.append(m)
	var cols := int(world.width() / chunk)
	var rows := int(world.depth() / chunk)
	var last := CHUNK_PROBES - 1.0
	for cy in rows:
		for cx in cols:
			var x_mid := world.x_min + (cx + 0.5) * chunk
			var y_mid := world.y_min + (cy + 0.5) * chunk
			var lo := 1e9
			var hi := -1e9
			for sy in CHUNK_PROBES:
				for sx in CHUNK_PROBES:
					var hh := world.h_at(x_mid + (sx / last - 0.5) * chunk, y_mid + (sy / last - 0.5) * chunk)
					lo = minf(lo, hh)
					hi = maxf(hi, hh)
			# ranges are measured from the node, so it sits at the chunk's own height; the shader
			# lifts the grid to the field whatever height the node has
			var mid_h := (lo + hi) * 0.5
			for k in levels.size():
				var mi := MeshInstance3D.new()
				mi.mesh = meshes[k]
				mi.material_override = material
				mi.position = WorldData.g3(x_mid, y_mid, mid_h)
				mi.custom_aabb = AABB(Vector3(-chunk * 0.5, lo - mid_h - CHUNK_BOX_PAD_M, -chunk * 0.5),
					Vector3(chunk, hi - lo + CHUNK_BOX_PAD_M * 2.0, chunk))
				mi.visibility_range_begin = float(levels[k][1])
				mi.visibility_range_end = float(levels[k][2])
				stage.add_child(mi)


func build_water() -> void:
	if world.water.is_empty():
		return
	var mat := ShaderMaterial.new()
	mat.shader = load("res://water.gdshader")
	mat.set_shader_parameter("height_tex", world.field_texture(world.height if world.bed.is_empty() else world.bed))
	mat.set_shader_parameter("water_tex", world.field_texture(world.water))
	mat.set_shader_parameter("surf", float(world.look.get("surf", 0.0)))
	if world.look.has("water_deep"):
		mat.set_shader_parameter("deep", world.look_color("water_deep", Color.BLACK))
		mat.set_shader_parameter("shallow", world.look_color("water_shallow", Color.BLACK))
	mat.set_shader_parameter("extent", world.extent4())
	mat.set_shader_parameter("posts", Vector2(world.nx, world.ny))
	_place_sheet(_sheet_mesh(WATER_GRID_M), mat, false)
	if world.look.get("horizon", "") == "sea":      # the sea runs on past the world's edge
		var far := PlaneMesh.new()
		far.size = Vector2(OPEN_SEA_SIZE_M, OPEN_SEA_SIZE_M)
		far.subdivide_width = 2
		far.subdivide_depth = 2
		var sea_mat := StandardMaterial3D.new()
		sea_mat.albedo_color = world.look_color("water_deep", Color(0.03, 0.13, 0.20))
		sea_mat.roughness = 0.08
		sea_mat.metallic_specular = 0.7
		var fm := MeshInstance3D.new()
		fm.mesh = far
		fm.material_override = sea_mat
		fm.position = world.centre(world.sea_level(0.0) - OPEN_SEA_DROP_M)
		fm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		stage.add_child(fm)


func _sheet_mesh(step: float) -> PlaneMesh:
	## A flat grid over the whole world with one vertex every `step` metres.
	var mesh := PlaneMesh.new()
	mesh.size = Vector2(world.width(), world.depth())
	mesh.subdivide_width = int(world.width() / step) - 1
	mesh.subdivide_depth = int(world.depth() / step) - 1
	return mesh


func _place_sheet(mesh: PlaneMesh, mat: Material, shadows: bool) -> void:
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = mat
	mi.position = world.centre(0.0)
	mi.custom_aabb = AABB(Vector3(-mesh.size.x * 0.5, SHEET_BOX_LOW_M, -mesh.size.y * 0.5),
		Vector3(mesh.size.x, SHEET_BOX_TALL_M, mesh.size.y))
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	stage.add_child(mi)
