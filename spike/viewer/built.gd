extends RefCounted
## Generated geometry that arrives as plain arrays in world coordinates: meshed volumes (cliffs,
## caves), built solids (walls, floors, steps) and their lights. Also owns the cutaway, which hides
## built solids from a given height up.

const WorldData := preload("res://world_data.gd")

## A solid whose base is this close under the cut height is treated as being at it, and hidden.
const CUT_MARGIN_M := 0.05
const DEFAULT_SURFACE_RGB := [0.8, 0.8, 0.8]
## Uniforms a volume takes from the terrain so that both draw the same ground.
const SHARED_GROUND_UNIFORMS := ["w0", "w1", "ground", "extent", "posts", "tile"]

var world: WorldData
var stage: Node3D
var colliders := false
var _cuttable: Array[MeshInstance3D] = []


func _init(world_data: WorldData, stage_node: Node3D, with_colliders: bool) -> void:
	world = world_data
	stage = stage_node
	colliders = with_colliders


func build_volumes(terrain_mat: ShaderMaterial) -> void:
	## Meshed volumes share the terrain's materials: ground on top, rock on faces and ceilings.
	var rock := 0
	for k in world.doc["ground"].size():
		if world.doc["ground"][k]["name"] == "rock":
			rock = k
	for vol in world.doc.get("volumes", []):
		var base := "%s/vol_%s" % [world.build_dir, vol["id"]]
		var arrays := _read_arrays(base)
		var count: int = arrays[Mesh.ARRAY_VERTEX].size()
		var lit := FileAccess.get_file_as_bytes(base + ".light.f32").to_float32_array()
		if lit.size() == count:       # baked sky exposure, one value per vertex
			var cols := PackedColorArray()
			cols.resize(count)
			for k in count:
				cols[k] = Color(lit[k], lit[k], lit[k], 1.0)
			arrays[Mesh.ARRAY_COLOR] = cols
		var mat := ShaderMaterial.new()
		mat.shader = load("res://volume.gdshader")
		for key in SHARED_GROUND_UNIFORMS:
			mat.set_shader_parameter(key, terrain_mat.get_shader_parameter(key))
		mat.set_shader_parameter("rock_layer", rock)
		mat.set_shader_parameter("sea_level", world.sea_level(-1000.0))
		if world.look.has("rock_a"):
			mat.set_shader_parameter("rock_a", world.look_color("rock_a", Color.WHITE))
			mat.set_shader_parameter("rock_b", world.look_color("rock_b", Color.WHITE))
			mat.set_shader_parameter("rock_tint", float(world.look.get("rock_tint", 1.0)))
			mat.set_shader_parameter("band_m", float(world.look.get("rock_band_m", 3.0)))
		var mesh := _place(arrays, mat).mesh
		_add_trimesh(mesh)


func build_meshes() -> void:
	## Built solids are coloured by a named surface.
	var surfaces: Dictionary = world.doc.get("surfaces", {})
	for m in world.doc.get("meshes", []):
		var arrays := _read_arrays("%s/mesh_%s" % [world.build_dir, m["id"]])
		var mat := StandardMaterial3D.new()
		mat.albedo_color = WorldData.rgb(surfaces.get(m["material"], {}).get("color", DEFAULT_SURFACE_RGB))
		mat.roughness = 0.9
		var mi := _place(arrays, mat)
		mi.set_meta("z0", float(m["z0"]))
		if m.get("collide", true):
			_add_trimesh(mi.mesh)
		if m.get("hidden", false):
			mi.visible = false
		else:
			_cuttable.append(mi)


func build_lights() -> void:
	for l in world.doc.get("lights", []):
		var lamp := OmniLight3D.new()
		lamp.position = WorldData.g3(l["pos"][0], l["pos"][1], l["pos"][2])
		lamp.light_color = WorldData.rgb(l["color"])
		lamp.omni_range = float(l["range"])
		lamp.light_energy = float(l["energy"])
		lamp.shadow_enabled = true
		stage.add_child(lamp)


func cut_above(cut: Variant) -> void:
	## A cutaway: hide everything built from this height up. Null shows everything.
	for node in _cuttable:
		node.visible = cut == null or float(node.get_meta("z0")) < float(cut) - CUT_MARGIN_M


func _read_arrays(base: String) -> Array:
	## Read `base`.verts.f32, .normals.f32 and .faces.u32 into surface arrays in engine axes.
	var v := FileAccess.get_file_as_bytes(base + ".verts.f32").to_float32_array()
	var n := FileAccess.get_file_as_bytes(base + ".normals.f32").to_float32_array()
	var f := FileAccess.get_file_as_bytes(base + ".faces.u32").to_int32_array()
	var verts := PackedVector3Array()
	var norms := PackedVector3Array()
	verts.resize(v.size() / 3)
	norms.resize(v.size() / 3)
	for k in verts.size():      # the same axis change as WorldData.g3, written out for speed
		verts[k] = Vector3(v[k * 3], v[k * 3 + 2], -v[k * 3 + 1])
		norms[k] = Vector3(n[k * 3], n[k * 3 + 2], -n[k * 3 + 1])
	for k in range(0, f.size(), 3):      # the mirror in the axis change flips the winding; flip it back
		var t := f[k + 1]
		f[k + 1] = f[k + 2]
		f[k + 2] = t
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = norms
	arrays[Mesh.ARRAY_INDEX] = f
	return arrays


func _place(arrays: Array, mat: Material) -> MeshInstance3D:
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = mat
	stage.add_child(mi)
	return mi


func _add_trimesh(mesh: Mesh) -> void:
	## Solid for the walking body. Both faces collide, so winding does not matter.
	if not colliders:
		return
	var shape := mesh.create_trimesh_shape()
	shape.backface_collision = true
	var cs := CollisionShape3D.new()
	cs.shape = shape
	var sb := StaticBody3D.new()
	sb.add_child(cs)
	stage.add_child(sb)
