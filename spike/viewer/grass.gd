extends RefCounted
## Grass: one small tuft mesh, scattered in square tiles that are built on demand around a point.
## Blades take the ground's colour at their root.

const WorldData := preload("res://world_data.gd")

const TILE_M := 16.0
## Shots draw grass further and thicker than walking can afford.
const RADIUS_SHOTS_M := 88.0
const RADIUS_WALK_M := 60.0
const PER_M2_SHOTS := 13.0
const PER_M2_WALK := 9.0
const BLADES_PER_TUFT := 3
const FADE_MARGIN_M := 12.0
## Roots sit a little under the surface so no gap shows on a slope.
const ROOT_SINK_M := 0.02
## A budget large enough to build every missing tile in one call.
const ALL_TILES := 100000

var world: WorldData
var radius := RADIUS_WALK_M
var per_m2 := PER_M2_WALK
var _root: Node3D
var _mesh: ArrayMesh
var _tiles: Dictionary = {}      # Vector2i tile -> its node, or null where no grass grew


func _init(world_data: WorldData, stage: Node3D, for_shots: bool) -> void:
	world = world_data
	if world.grass.is_empty():      # a world without a grass field gets no node and no mesh
		return
	if for_shots:
		radius = RADIUS_SHOTS_M
		per_m2 = PER_M2_SHOTS
	_root = Node3D.new()
	stage.add_child(_root)
	_mesh = _tuft_mesh()


func _tuft_mesh() -> ArrayMesh:
	## A few leaning, tapering blades fanned around the root, one unit tall and one unit wide.
	var verts := PackedVector3Array()
	var uvs := PackedVector2Array()
	var idx := PackedInt32Array()
	# each row of a blade is [height along it, half width there]
	var rows := [[0.0, 0.5], [0.45, 0.42], [0.8, 0.24], [1.0, 0.0]]
	for b in BLADES_PER_TUFT:
		var ang := TAU * b / float(BLADES_PER_TUFT) + 0.4
		var side := Vector3(cos(ang), 0, sin(ang))
		var lean := Vector3(-sin(ang), 0, cos(ang)) * 0.28
		var off := lean * 0.35
		var base := verts.size()
		for row in rows:
			var y: float = row[0]
			var hw: float = row[1]
			var c := off + lean * y * y + Vector3(0, y, 0)
			verts.append(c - side * hw)
			verts.append(c + side * hw)
			uvs.append(Vector2(0, y))
			uvs.append(Vector2(1, y))
		for q in rows.size() - 1:
			var o := base + q * 2
			idx.append_array(PackedInt32Array([o, o + 1, o + 2, o + 1, o + 3, o + 2]))
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = idx
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mat := ShaderMaterial.new()
	mat.shader = load("res://grass.gdshader")
	mesh.surface_set_material(0, mat)
	return mesh


func grow_around(wx: float, wy: float, budget: int) -> int:
	## Build missing grass tiles near a world position, nearest first. Returns how many are still missing.
	if world.grass.is_empty():
		return 0
	var cx := int(floor((wx - world.x_min) / TILE_M))
	var cy := int(floor((wy - world.y_min) / TILE_M))
	var reach := int(ceil(radius / TILE_M))
	var max_tx := int(world.width() / TILE_M)
	var max_ty := int(world.depth() / TILE_M)
	var todo: Array = []
	for ty in range(cy - reach, cy + reach + 1):
		for tx in range(cx - reach, cx + reach + 1):
			if tx < 0 or ty < 0 or tx >= max_tx or ty >= max_ty or _tiles.has(Vector2i(tx, ty)):
				continue
			var d := Vector2(tx - cx, ty - cy).length()
			if d * TILE_M <= radius:
				todo.append([d, tx, ty])
	todo.sort_custom(func(a, b): return a[0] < b[0])
	var done := 0
	for t in todo:
		if done >= budget:
			break
		_grow_tile(t[1], t[2])
		done += 1
	return todo.size() - done


func _grow_tile(tx: int, ty: int) -> void:
	var key := Vector2i(tx, ty)
	if _tiles.has(key):
		return
	_tiles[key] = null
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(key)
	var x_lo := world.x_min + tx * TILE_M
	var y_lo := world.y_min + ty * TILE_M
	var n := int(TILE_M * TILE_M * per_m2)
	var xfs: Array[Transform3D] = []
	var cols: Array[Color] = []
	for k in n:
		var x := x_lo + rng.randf() * TILE_M
		var y := y_lo + rng.randf() * TILE_M
		if rng.randf() > world.sample(world.grass, x, y):
			continue
		var tall := 0.28 + 0.34 * rng.randf() * rng.randf() + 0.2 * rng.randf()
		var wide := 0.09 + 0.05 * rng.randf()
		var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(wide, tall, wide))
		xfs.append(Transform3D(basis, WorldData.g3(x, y, world.h_at(x, y) - ROOT_SINK_M)))
		var c := world.ground_color(x, y)
		var j := 0.9 + 0.2 * rng.randf()
		cols.append(Color(c.r * j, c.g * j, c.b * (0.92 + 0.12 * rng.randf())))
	if xfs.is_empty():
		return
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.use_custom_data = true
	mm.mesh = _mesh
	mm.instance_count = xfs.size()
	for k in xfs.size():
		mm.set_instance_transform(k, xfs[k])
		mm.set_instance_color(k, cols[k])
		mm.set_instance_custom_data(k, Color(rng.randf(), 0, 0, 0))      # the blade's own wind phase
	var mmi := MultiMeshInstance3D.new()
	mmi.multimesh = mm
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mmi.visibility_range_end = radius + TILE_M
	mmi.visibility_range_end_margin = FADE_MARGIN_M
	mmi.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
	_root.add_child(mmi)
	_tiles[key] = mmi
