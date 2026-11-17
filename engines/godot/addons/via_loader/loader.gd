extends RefCounted
## The Godot loader of spec/build.md 15.1: reads one build directory (format via-build-1) and makes
## one node tree. The editor plugin saves that tree as a scene; the sheet tool makes it at start.
## Both run this code, so compile-time export and runtime loading differ only in when the tree is
## saved. It reads only the directory and the files the manifest names under `pack_root`; it never
## reads world text and never resolves a name.
##
##   var loader := Loader.new()
##   var root := loader.load_build("/path/to/build/ford", {"water": "tag:water_level"})
##
## Options are the loader's own settings (15.1), never world text:
##   fields       Array of field selectors (a name, role:<role>, tag:<tag>): kept as data in the
##                root's metadata "fields".
##   water        One selector naming a water surface height field (6.4) to draw as water.
##   features     bool: make nodes for features.
##   diagnostics  A path: write the loader's diagnostics there as diagnostics.md 7's document.
## A bad option is not a diagnostic of the build: `option_error` says what, and nothing is made.

const Npy := preload("res://addons/via_loader/npy.gd")
const Diagnostics := preload("res://addons/via_loader/diagnostics.gd")
const Primitives := preload("res://addons/via_loader/primitives.gd")
const TERRAIN_SHADER := preload("res://addons/via_loader/terrain.gdshader")
const INSTANCE_SHADER := preload("res://addons/via_loader/instance.gdshader")

const FORMAT := "via-build-1"
## The most materials one chunk's shader draws: the layers of one Texture2DArray every renderer
## Godot ships allows (OpenGL ES 3 guarantees 256). A chunk naming more is load.too_many_materials.
const MAX_MATERIALS := 256
## Material images are resampled to one size so they share one Texture2DArray.
const ALBEDO_SIZE := 256
## A post with every weight 0 holds no material, and a world with no material field has none: this
## loader's default (build.md 6.2). A plain mid grey (sRGB 0.5, linear 0.214); exposure is the sheet
## rig's (rig.gd), never set by darkening a colour.
const DEFAULT_GROUND := Color(0.5, 0.5, 0.5)
const WATER_COLOUR := Color(0.12, 0.3, 0.38, 0.85)
## The water's opacity: WATER_CLEAR at depth 0, thickening by WATER_THICKEN per metre of depth up to
## WATER_COLOUR's alpha.
const WATER_CLEAR := 0.3
const WATER_THICKEN := 1.2
## The least a dry post at the water's edge stands above the surface drawn there, metres
## (_shore_level), so the edge always falls between posts.
const SHORE_GAP := 0.02
## The `paths` option (build.md 15.1): a path feature drawn as a strip on the ground, until a world
## paints its ways. Its `width` attribute in metres when it has one, else PATH_WIDTH; raised
## PATH_LIFT over the ground mesh so it does not hide it; linear, as a pack colour is. Paths that
## share an end point are joined there by a disc.
const PATH_COLOUR := Color(0.42, 0.33, 0.2)
const PATH_WIDTH := 1.0
const PATH_LIFT := 0.04
## Path ends this close, in metres, are one meeting (geometry.md 2.7: ends meet exactly), drawn with
## a round join of JOIN_SEGMENTS sides.
const JOIN_TOLERANCE := 0.001
const JOIN_SEGMENTS := 32
## Degrees a path's line turns at a vertex beyond which its strip adds a disc there (_strip).
const TURN_DISC := 30.0
const OPTION_NAMES := ["diagnostics", "features", "fields", "paths", "water"]

var diags := Diagnostics.new()
var options := {"fields": [], "water": "", "features": false, "paths": [], "diagnostics": ""}
## Where the pack's root is when the manifest's `pack_root` is null (build.md 4).
var pack_dir := ""
var option_error := ""

var _dir := ""
var _manifest: Dictionary = {}
var _nx := 0
var _ny := 0
var _x0 := 0.0
var _y0 := 0.0
var _res := 1.0
var _height := PackedFloat32Array()
## The terrain's cut (build.md 6.5): one byte per quad, (ny - 1) by (nx - 1), 1 where cut; empty
## when the build writes none (before M5), so no quad is cut.
var _cut := PackedByteArray()
var _water := PackedFloat32Array()
var _asset_mesh := {}
var _asset_material := {}
var _albedo := {}
var _water_material: StandardMaterial3D
var _features_cache = null
## Streaming (build.md 15.1, M6): `prepare` makes the root without chunks or generated meshes, and
## stream.gd makes each from `make_chunk` and `make_mesh` as a walker nears it.
var _streamed := false
var _material_field := ""
var _cols := {}
var _with_water := false


# --- options -------------------------------------------------------------------------------------

static func valid_selector(s: String) -> bool:
	var body := s
	if s.begins_with("role:"):
		body = s.trim_prefix("role:")
	elif s.begins_with("tag:"):
		body = s.trim_prefix("tag:")
	var re := RegEx.create_from_string("^[a-z][a-z0-9_]*$")
	return re.search(body) != null


static func valid_feature_selector(s: String) -> bool:
	## A feature's name or `kind:<kind>` (the `paths` option).
	var re := RegEx.create_from_string("^[a-z][a-z0-9_]*$")
	return re.search(s.trim_prefix("kind:")) != null


static func parse_option(opts: Dictionary, name: String, value: String) -> String:
	## Sets one option from text, as the sheet tool's --option NAME=VALUE gives it. Returns "" or
	## what is wrong. A list's items are separated by `,`; `features` takes true or false.
	match name:
		"paths":
			var sel: Array = []
			for s in value.split(",", false):
				if not valid_feature_selector(s):
					return "paths: `%s` is not a feature selector (a name or kind:<kind>)" % s
				sel.append(s)
			opts["paths"] = sel
		"fields":
			var list: Array = []
			for s in value.split(",", false):
				if not valid_selector(s):
					return "fields: `%s` is not a field selector (a name, role:<role> or tag:<tag>)" % s
				list.append(s)
			opts["fields"] = list
		"water":
			if not valid_selector(value):
				return "water: `%s` is not a field selector (a name, role:<role> or tag:<tag>)" % value
			opts["water"] = value
		"features":
			if value != "true" and value != "false":
				return "features takes true or false, not `%s`" % value
			opts["features"] = value == "true"
		"diagnostics":
			opts["diagnostics"] = value
		_:
			return "no loader option `%s`; the options are %s" % [name, ", ".join(OPTION_NAMES)]
	return ""


static func node_name(name: String) -> String:
	## An instance or bookmark name as a node name: `.` becomes `-`. Names never hold `-`, so the
	## mapping reverses (build.md 15.1). `#` and `_` are kept.
	return name.replace(".", "-")


static func g(x: float, y: float, z: float) -> Vector3:
	## World frame (x east, y north, z up) to Godot (X, Y, Z) = (x, z, -y).
	return Vector3(x, z, -y)


# --- loading -------------------------------------------------------------------------------------

func load_build(build_dir: String, opts := {}) -> Node3D:
	## Loads and reports: to `options.diagnostics` when set, else to Godot's output.
	var root := make(build_dir, opts)
	if options.diagnostics != "":
		diags.write(options.diagnostics)
	else:
		diags.print_all()
	return root


func make(build_dir: String, opts := {}) -> Node3D:
	## Loads without reporting; diagnostics stay in `diags`. Null when no tree can be made.
	for k in opts:
		if not k in OPTION_NAMES:
			option_error = "no loader option `%s`" % k
			return null
		options[k] = opts[k]
	_dir = build_dir.trim_suffix("/")
	if not _read_manifest():
		return null
	var root := Node3D.new()
	root.name = String(_manifest.get("world", "world"))
	root.set_meta("id", _manifest.get("id", ""))
	root.set_meta("format", _manifest.format)
	root.set_meta("world", _manifest.get("world", ""))
	root.set_meta("frame", _manifest.frame)
	var fields: Dictionary = _manifest.get("fields", {})
	var surface := _field_with_role(fields, "surface", "scalar")
	var material := _field_with_role(fields, "material", "weights")
	var water := ""
	if options.water != "":
		var hits := _select(fields, options.water)
		var scalars := hits.filter(func(n): return fields[n].get("type") == "scalar")
		if scalars.size() != 1:
			option_error = "water: `%s` selects %d scalar fields in this build (%s); it must select one" % [
				options.water, scalars.size(), ", ".join(hits)]
			root.free()
			return null
		water = scalars[0]
	if surface != "":
		_height = _scalar(surface, fields[surface])
		_cut = _cut_of(fields[surface])
	if water != "":
		_water = _scalar(water, fields[water])
		if _water.is_empty():
			water = ""
	var drawn := {}
	if not _height.is_empty():
		drawn[surface] = true
	if water != "":
		drawn[water] = true
	if material != "":
		drawn[material] = true
	var cols := _instance_columns()
	_material_field = material
	_cols = cols
	_with_water = not water.is_empty()
	if _streamed:
		_warm()
	else:
		for chunk in _manifest.get("chunks", []):
			root.add_child(_chunk(chunk, material, cols, not water.is_empty()))
		_meshes(root)
	_features(root)
	if not _paths(root):
		root.free()
		return null
	_bookmarks(root)
	_meets(root)
	var kept := _keep_fields(root, fields)
	var not_drawn: Array = []
	for name in fields:
		if not drawn.has(name) and not kept.has(name):
			not_drawn.append(name)
	if not not_drawn.is_empty():
		not_drawn.sort()
		diags.add("load.fields_not_drawn", "world", "manifest.json",
			"%d field(s) neither drawn nor kept: %s" % [not_drawn.size(), ", ".join(not_drawn)],
			"name them in the `fields` option to keep them as data, or `water` to draw a water surface",
			{"fields": not_drawn})
	return root


func prepare(build_dir: String, opts := {}) -> Node3D:
	## As `make`, but the root holds no chunk node and no generated mesh: stream.gd makes each with
	## make_chunk and make_mesh. Everything chunks share (asset meshes and materials, material
	## images, the water material) is made here, so a chunk can be made on another thread while
	## nothing else touches the loader.
	_streamed = true
	return make(build_dir, opts)


func _warm() -> void:
	for chunk in _manifest.get("chunks", []):
		for asset in chunk.get("instances", {}):
			_asset_mesh_of(asset)
		for w in chunk.get("weights", {}).values():
			for m in w.get("materials", []):
				_material_image(m, _manifest.get("materials", {}).get(m, {}))
	if _with_water:
		water_material()


func chunk_list() -> Array:
	return _manifest.get("chunks", [])


func make_chunk(n: int) -> Node3D:
	## Chunk n of the manifest's list as `make` adds it: `c_<cx>_<cy>` with its ground, water,
	## instances and their bodies. Not in the tree, so it may be made on a worker thread.
	return _chunk(_manifest.chunks[n], _material_field, _cols, _with_water)


## Ours: the far ground's posts lie at most FAR_STEP metres apart. Eight quads a chunk side was
## 8 m on a 64 m chunk and 16 m on atlas's 128 m chunks, where a scarp's rock face shrank into
## grass past the load radius (M6 close: along_north 2.45% of pixels off whole at 96 m).
const FAR_STEP := 8.0


func make_far_ground(step_m := FAR_STEP) -> MeshInstance3D:
	## Every chunk's ground with its posts at most `step_m` metres apart (every few posts, its
	## first and last posts kept, so neighbours meet on their shared edge posts), as one mesh, not collided and with no
	## cut, each post coloured by its chunk's material weights there as terrain.gdshader blends
	## them (no texture): what stream.gd shows of the chunks that are not loaded, so the world does
	## not end at the load radius (build.md 15.1, Streaming). Each vertex's UV holds its chunk's
	## `at` plus a half; the shader hides a chunk whose texel of the `loaded` texture (one per
	## chunk, `[cx, cy]`, R8) is set, so the whole far ground is one draw call. Null without a
	## surface field.
	if _height.is_empty():
		return null
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var tints := PackedColorArray()
	var uvs := PackedVector2Array()
	var idx := PackedInt32Array()
	var wverts := PackedVector3Array()
	var wcolours := PackedColorArray()
	var wuvs := PackedVector2Array()
	var ground := DEFAULT_GROUND.srgb_to_linear()
	for chunk in _manifest.get("chunks", []):
		var p: Array = chunk.get("posts", [0, 0, 0, 0])
		var at: Array = chunk.get("at", [0, 0])
		var is_ := _far_steps(int(p[0]), int(p[2]), ceili((int(p[2]) - int(p[0])) * _res / step_m))
		var js := _far_steps(int(p[1]), int(p[3]), ceili((int(p[3]) - int(p[1])) * _res / step_m))
		# The weights planes, [m][(j - j0) * cols + (i - i0)], and each material's colour.
		var w: Dictionary = chunk.get("weights", {}).get(_material_field, {}) if _material_field != "" else {}
		var planes := PackedByteArray()
		var colours: Array[Color] = []
		var cols := int(p[2]) - int(p[0]) + 1
		var plane := cols * (int(p[3]) - int(p[1]) + 1)
		if w.get("file") is String:
			var arr := Npy.read(_dir + "/" + String(w.file))
			var names: Array = w.get("materials", [])
			if Npy.expect(arr, "|u1", [names.size(), int(p[3]) - int(p[1]) + 1, cols]) == "":
				planes = arr.bytes
				for m in names:
					var c: Array = _manifest.get("materials", {}).get(m, {}).get("colour", [0.8, 0.8, 0.8])
					colours.append(Color(c[0], c[1], c[2]))
		var uv := Vector2(int(at[0]) + 0.5, int(at[1]) + 0.5)
		var base := verts.size()
		for j in js:
			for i in is_:
				verts.append(g(_x0 + i * _res, _y0 + j * _res, _h(i, j)))
				var dx := (_h(i + 1, j) - _h(i - 1, j)) / ((mini(i + 1, _nx - 1) - maxi(i - 1, 0)) * _res)
				var dy := (_h(i, j + 1) - _h(i, j - 1)) / ((mini(j + 1, _ny - 1) - maxi(j - 1, 0)) * _res)
				normals.append(g(-dx, -dy, 1.0).normalized())
				uvs.append(uv)
				var c := Color(0, 0, 0)
				var sum := 0.0
				for m in colours.size():
					var wt := planes[m * plane + (j - int(p[1])) * cols + (i - int(p[0]))] / 255.0
					c += colours[m] * wt
					sum += wt
				tints.append(c / sum if sum > 0.001 else ground)   # as terrain.gdshader blends them
		for k in _grid_indices(is_.size(), js.size(), func(_c, _r): return true):
			idx.append(base + k)
		_far_skirts(verts, normals, tints, uvs, idx, base, is_, js)
		if _with_water:
			_far_water(wverts, wcolours, wuvs, is_, js, uv)
	var mesh := ArrayMesh.new()
	if not idx.is_empty():
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = verts
		arrays[Mesh.ARRAY_NORMAL] = normals
		arrays[Mesh.ARRAY_COLOR] = tints
		arrays[Mesh.ARRAY_TEX_UV] = uvs
		arrays[Mesh.ARRAY_INDEX] = idx
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var node := MeshInstance3D.new()
	node.name = "far_ground"
	node.mesh = mesh
	node.material_override = _far_material(0.95)
	if not wverts.is_empty():
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = wverts
		var up := PackedVector3Array()
		up.resize(wverts.size())
		up.fill(Vector3.UP)
		arrays[Mesh.ARRAY_NORMAL] = up
		arrays[Mesh.ARRAY_COLOR] = wcolours
		arrays[Mesh.ARRAY_TEX_UV] = wuvs
		var wmesh := ArrayMesh.new()
		wmesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		var water := MeshInstance3D.new()
		water.name = "far_water"
		water.mesh = wmesh
		water.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var shader := Shader.new()
		shader.code = "shader_type spatial;\nrender_mode blend_mix, cull_disabled;\nuniform sampler2D loaded : filter_nearest, repeat_disable;\nuniform vec4 colour : source_color;\nvoid fragment() {\n\tif (texelFetch(loaded, ivec2(floor(UV)), 0).r > 0.5) {\n\t\tdiscard;\n\t}\n\tALBEDO = colour.rgb;\n\tALPHA = COLOR.a;\n\tROUGHNESS = 0.25;\n\tSPECULAR = 0.3;\n}\n"
		var mat := ShaderMaterial.new()
		mat.shader = shader
		mat.set_shader_parameter("colour", Color(WATER_COLOUR, 1.0))
		water.material_override = mat
		node.add_child(water)
	return node


## Ours: a far quad's water is found from FAR_WATER_SAMPLES by FAR_WATER_SAMPLES posts of its cell
## (its corners included), so a tarn or a stream narrower than a far quad still shows past the load
## radius; a skirt hangs FAR_SKIRT below the deepest a chunk's fine edge dips under its far edge.
const FAR_WATER_SAMPLES := 5
const FAR_SKIRT := 0.1


func _far_skirts(verts: PackedVector3Array, normals: PackedVector3Array, tints: PackedColorArray,
		uvs: PackedVector2Array, idx: PackedInt32Array, base: int, is_: Array[int], js: Array[int]) -> void:
	## Each far edge of a chunk (posts every few, straight between) passes over the posts of the
	## loaded neighbour's fine edge where they dip under it, a slit the sky shows through from the
	## loaded side. A skirt hangs from each far edge segment to below the deepest such dip: the
	## segment's own vertices, and the same less the dip plus FAR_SKIRT (0.1 m when it dips nowhere).
	var cols := is_.size()
	var south := []
	var north := []
	for c in cols:
		south.append(c)
		north.append((js.size() - 1) * cols + c)
	var west := []
	var east := []
	for r in js.size():
		west.append(r * cols)
		east.append(r * cols + cols - 1)
	for line in [south, north, west, east]:
		for k in line.size() - 1:
			var a: int = line[k]
			var b: int = line[k + 1]
			var pa := Vector2i(is_[a % cols], js[a / cols])
			var pb := Vector2i(is_[b % cols], js[b / cols])
			var ha: float = verts[base + a].y
			var hb: float = verts[base + b].y
			var dip := 0.0
			var n := maxi(absi(pb.x - pa.x), absi(pb.y - pa.y))
			for s in range(1, n):
				var t := float(s) / n
				var p := pa + (pb - pa) * s / n
				dip = maxf(dip, lerpf(ha, hb, t) - _h(p.x, p.y))
			var v0 := verts.size()
			for q in [base + a, base + b]:
				verts.append(verts[q])
				normals.append(normals[q])
				tints.append(tints[q])
				uvs.append(uvs[q])
			for q in [base + a, base + b]:
				verts.append(verts[q] - Vector3(0, dip + FAR_SKIRT, 0))
				normals.append(normals[q])
				tints.append(tints[q])
				uvs.append(uvs[q])
			# both faces: a skirt is seen from either side of its edge
			idx.append_array([v0, v0 + 1, v0 + 3, v0, v0 + 3, v0 + 2, v0, v0 + 3, v0 + 1, v0, v0 + 2, v0 + 3])


func _far_water(wverts: PackedVector3Array, wcolours: PackedColorArray, wuvs: PackedVector2Array,
		is_: Array[int], js: Array[int], uv: Vector2) -> void:
	## A chunk's far water: for each far quad, the posts of FAR_WATER_SAMPLES a side over it (its
	## corners included); where any is wet (water above ground, as the near water), a quad over the
	## far quad with each corner at the water level of the wet sample nearest it, its alpha the
	## near water's for the mean depth of the wet samples. The ground hides it where it stands
	## higher, so the water's edge falls where the far ground crosses it.
	for b in js.size() - 1:
		for a in is_.size() - 1:
			var i0: int = is_[a]
			var i1: int = is_[a + 1]
			var j0: int = js[b]
			var j1: int = js[b + 1]
			var wet: Array = []   # [Vector2i post, level]
			var depth := 0.0
			for sj in FAR_WATER_SAMPLES:
				for si in FAR_WATER_SAMPLES:
					var i := i0 + roundi(float(i1 - i0) * si / (FAR_WATER_SAMPLES - 1))
					var j := j0 + roundi(float(j1 - j0) * sj / (FAR_WATER_SAMPLES - 1))
					var k := j * _nx + i
					if _water[k] > _height[k]:
						wet.append([Vector2i(i, j), _water[k]])
						depth += _water[k] - _height[k]
			if wet.is_empty():
				continue
			depth /= wet.size()
			var alpha := clampf(WATER_CLEAR + depth * WATER_THICKEN, WATER_CLEAR, WATER_COLOUR.a)
			var corner := func(i: int, j: int) -> Vector3:
				var best: float = wet[0][1]
				var bd := INF
				for w in wet:
					var d: float = Vector2(w[0] - Vector2i(i, j)).length_squared()
					if d < bd:
						bd = d
						best = w[1]
				return g(_x0 + i * _res, _y0 + j * _res, best)
			var sw: Vector3 = corner.call(i0, j0)
			var se: Vector3 = corner.call(i1, j0)
			var nw: Vector3 = corner.call(i0, j1)
			var ne: Vector3 = corner.call(i1, j1)
			for v in [sw, ne, se, sw, nw, ne]:
				wverts.append(v)
				wcolours.append(Color(1, 1, 1, alpha))
				wuvs.append(uv)


func _far_material(roughness: float) -> ShaderMaterial:
	## The far layers' material: the vertex colour, linear as the pack's colours are (assets.md
	## 2.6), drawn as terrain.gdshader draws a colour, through linear.gdshaderinc; a fragment whose
	## chunk (its UV, the chunk's `at` plus a half) is set in the `loaded` texture is discarded.
	var shader := Shader.new()
	shader.code = "shader_type spatial;\n#include \"res://addons/via_loader/linear.gdshaderinc\"\nuniform sampler2D loaded : filter_nearest, repeat_disable;\nvoid fragment() {\n\tif (texelFetch(loaded, ivec2(floor(UV)), 0).r > 0.5) {\n\t\tdiscard;\n\t}\n\tALBEDO = albedo_from_linear(COLOR.rgb);\n\tROUGHNESS = %.2f;\n}\n" % roughness
	var mat := ShaderMaterial.new()
	mat.shader = shader
	return mat


func make_far_instances() -> MeshInstance3D:
	## Every chunk's instances as boxes, as one mesh: each instance whose asset is drawn and has
	## `bounds` (assets.md 2.3) is its asset's outline (_far_parts: a box per height band of the
	## asset's own surface) placed by its row (build.md 8, scale included), four sides and a top
	## each (no bottom), flat-shaded: a primitive's or stand-in's `colour` times the row's tint, as
	## instance.gdshader draws it; a model's band the mean albedo of its surface there. Not collided. What stream.gd shows of a chunk's instances
	## while the chunk is not loaded, in the far ground's way (one draw call, a chunk hidden by its
	## texel of the `loaded` texture), so houses on a far hill stand past the load radius (M6 slice
	## 2). The reader's level of detail; ours. Null when no chunk holds an instance drawn.
	var cols := _cols
	if not cols.has("names"):
		return null
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var tints := PackedColorArray()
	var uvs := PackedVector2Array()
	var colour := {}   # asset -> [linear colour, uses tint] or null when not drawn
	for chunk in _manifest.get("chunks", []):
		var at: Array = chunk.get("at", [0, 0])
		var uv := Vector2(int(at[0]) + 0.5, int(at[1]) + 0.5)
		var groups: Dictionary = chunk.get("instances", {})
		var assets := groups.keys()
		assets.sort()
		for asset in assets:
			if not colour.has(asset):
				colour[asset] = _far_parts(asset)
			var parts = colour[asset]
			if parts == null:
				continue
			var r: Array = groups[asset]
			for k in int(r[1]):
				var n := int(r[0]) + k
				if n < 0 or n >= int(cols.count):
					continue
				var t := _instance_transform(cols, n, true)
				for part in parts:
					var tint: Color = part[1] * (cols.tint[n] if part[2] else 1.0)
					_far_frustum(verts, normals, tints, uvs, t, part, Color(tint.r, tint.g, tint.b), uv)
	if verts.is_empty():
		return null
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = tints
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var node := MeshInstance3D.new()
	node.name = "far_instances"
	node.mesh = mesh
	node.material_override = _far_material(0.85)
	return node


## Ours: a far instance is its asset's outline in FAR_BANDS bands of its height. One box of the
## bounds drew a wood as a wall of solid blocks from the ground to the crowns (M6 slice 2, atlas at
## 96 m); a box per band follows a stem under a crown, a crown narrowing to its top, and is a box
## still for a box. M7: each band is a frustum between the outlines of the asset's surface about
## its two ends (`far_ring`), each outline an octagon (its box cut by its diagonal extents), so a
## crown is round and a cone tapers; a box's outlines are its box.
const FAR_BANDS := 3


func _far_parts(asset: String):
	## [[bounds (asset frame), linear colour, whether the row's tint multiplies it, bottom ring,
	## top ring], ...], or null for an asset not drawn or without bounds. The asset's height (its
	## bounds' z) is cut in FAR_BANDS equal bands; the ring at each of the FAR_BANDS + 1 band ends
	## z_k is the 8-sided outline ([x0, y0, x1, y1, least and most x + y, least and most x - y]) of
	## the surface within half a band of z_k (each triangle clipped there), and band k runs from
	## ring k to ring k + 1 (its bounds the box of both). A ring no surface reaches is its
	## neighbour's. A model's band takes the mean albedo of its triangles whose centre lies in it,
	## by area (else the model's mean); a primitive's or stand-in's every band its `colour`.
	## Neighbouring bands of the same rings and colour are one.
	var a: Dictionary = _manifest.get("assets", {}).get(asset, {})
	var b: Array = a.get("bounds", [])
	var mesh := _asset_mesh_of(asset)
	if mesh == null or b.size() != 6:
		return null
	var model: bool = a.get("source") == "model" and not a.get("stand_in", false)
	var c: Array = a.get("colour", [0.8, 0.8, 0.8])
	var flat := Color(c[0], c[1], c[2])
	var z0: float = b[2]
	var dz: float = (float(b[5]) - z0) / FAR_BANDS
	var whole := [b[0], b[1], b[3], b[4], float(b[0]) + float(b[1]), float(b[3]) + float(b[4]), float(b[0]) - float(b[4]), float(b[3]) - float(b[1])]
	if dz <= 0.0:
		return [[b, flat, not model, whole, whole]]
	var lo: Array[Vector2] = []
	var hi: Array[Vector2] = []
	var dlo: Array[Vector2] = []   # per ring: the least (x + y, x - y) of the surface about it
	var dhi: Array[Vector2] = []
	var sum: Array[Color] = []
	var area: Array[float] = []
	for k in FAR_BANDS + 1:
		lo.append(Vector2(INF, INF))
		hi.append(Vector2(-INF, -INF))
		dlo.append(Vector2(INF, INF))
		dhi.append(Vector2(-INF, -INF))
		sum.append(Color(0, 0, 0))
		area.append(0.0)
	var all_sum := Color(0, 0, 0)
	var all_area := 0.0
	for si in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(si)
		var v: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var ix = arrays[Mesh.ARRAY_INDEX]
		var m := mesh.surface_get_material(si) as BaseMaterial3D
		var albedo := m.albedo_color.srgb_to_linear() if m != null else DEFAULT_GROUND.srgb_to_linear()
		var tris: int = (ix.size() if ix is PackedInt32Array and not ix.is_empty() else v.size()) / 3
		for t in tris:
			var q: Array[Vector3] = []
			for e in 3:
				var gv: Vector3 = v[ix[3 * t + e]] if ix is PackedInt32Array and not ix.is_empty() else v[3 * t + e]
				q.append(Vector3(gv.x, -gv.z, gv.y))   # Godot axes to the asset frame
			var ar := (q[1] - q[0]).cross(q[2] - q[0]).length() / 2.0
			var zc := (q[0].z + q[1].z + q[2].z) / 3.0
			var kc := clampi(int((zc - z0) / dz), 0, FAR_BANDS - 1)
			sum[kc] += albedo * ar
			area[kc] += ar
			all_sum += albedo * ar
			all_area += ar
			for k in FAR_BANDS + 1:
				var bz0 := z0 + (k - 0.5) * dz
				var bz1 := bz0 + dz
				var pts: Array[Vector2] = []
				for e in 3:
					var p0 := q[e]
					var p1 := q[(e + 1) % 3]
					if p0.z >= bz0 - 1e-6 and p0.z <= bz1 + 1e-6:
						pts.append(Vector2(p0.x, p0.y))
					for zc2 in [bz0, bz1]:
						if (p0.z - zc2) * (p1.z - zc2) < 0.0:
							var f: float = (zc2 - p0.z) / (p1.z - p0.z)
							var x := p0.lerp(p1, f)
							pts.append(Vector2(x.x, x.y))
				for pt in pts:
					lo[k] = lo[k].min(pt)
					hi[k] = hi[k].max(pt)
					var d := Vector2(pt.x + pt.y, pt.x - pt.y)
					dlo[k] = dlo[k].min(d)
					dhi[k] = dhi[k].max(d)
	var mean := all_sum / all_area if all_area > 0.0 else DEFAULT_GROUND.srgb_to_linear()
	var rings: Array = []
	for k in FAR_BANDS + 1:
		rings.append([lo[k].x, lo[k].y, hi[k].x, hi[k].y, dlo[k].x, dhi[k].x, dlo[k].y, dhi[k].y] if lo[k].x <= hi[k].x else null)
	if rings.all(func(r): return r == null):
		return null
	for k in FAR_BANDS + 1:   # a ring no surface reaches takes its nearest reached neighbour's
		var d := 1
		while rings[k] == null:
			if k - d >= 0 and rings[k - d] != null:
				rings[k] = rings[k - d].duplicate()
			elif k + d <= FAR_BANDS and rings[k + d] != null:
				rings[k] = rings[k + d].duplicate()
			d += 1
	var parts: Array = []
	for k in FAR_BANDS:
		var col := flat
		if model:
			var mc := sum[k] / area[k] if area[k] > 0.0 else mean
			col = Color(mc.r, mc.g, mc.b)
		var r0: Array = rings[k]
		var r1: Array = rings[k + 1]
		var box := [minf(r0[0], r1[0]), minf(r0[1], r1[1]), z0 + k * dz, maxf(r0[2], r1[2]), maxf(r0[3], r1[3]), z0 + (k + 1) * dz]
		if not parts.is_empty():
			var last: Array = parts[-1]
			var lb: Array = last[0]
			if _same_ring(last[3], last[4]) and _same_ring(last[4], r0) and _same_ring(r0, r1) \
					and last[1].is_equal_approx(col) and absf(lb[5] - box[2]) < 1e-6:
				lb[5] = box[5]
				continue
		parts.append([box, col, not model, r0, r1])
	return parts if not parts.is_empty() else null


static func _same_ring(a: Array, b: Array) -> bool:
	return range(8).all(func(i): return absf(float(a[i]) - float(b[i])) < 0.01)


static func _far_box(verts: PackedVector3Array, normals: PackedVector3Array, tints: PackedColorArray,
		uvs: PackedVector2Array, t: Transform3D, b: Array, colour: Color, uv: Vector2) -> void:
	## The box of bounds b (asset frame) placed by t: four sides and the top, two triangles each,
	## wound clockwise seen from outside (Godot's front faces).
	var p := func(x, y, z): return t * g(x, y, z)
	var x0: float = b[0]
	var y0: float = b[1]
	var z0: float = b[2]
	var x1: float = b[3]
	var y1: float = b[4]
	var z1: float = b[5]
	var faces := [
		[p.call(x0, y0, z1), p.call(x1, y0, z1), p.call(x1, y1, z1), p.call(x0, y1, z1)],   # top
		[p.call(x0, y0, z0), p.call(x1, y0, z0), p.call(x1, y0, z1), p.call(x0, y0, z1)],   # south
		[p.call(x1, y1, z0), p.call(x0, y1, z0), p.call(x0, y1, z1), p.call(x1, y1, z1)],   # north
		[p.call(x0, y1, z0), p.call(x0, y0, z0), p.call(x0, y0, z1), p.call(x0, y1, z1)],   # west
		[p.call(x1, y0, z0), p.call(x1, y1, z0), p.call(x1, y1, z1), p.call(x1, y0, z1)],   # east
	]
	var centre: Vector3 = t * g((x0 + x1) / 2.0, (y0 + y1) / 2.0, (z0 + z1) / 2.0)
	for q in faces:
		var nrm: Vector3 = (q[1] - q[0]).cross(q[2] - q[0]).normalized()
		var mid: Vector3 = (q[0] + q[2]) / 2.0
		if nrm.dot(mid - centre) < 0.0:
			nrm = -nrm
		for tri in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]:
			var a: Vector3 = tri[0]
			var bb: Vector3 = tri[1]
			var cc: Vector3 = tri[2]
			if (bb - a).cross(cc - a).dot(nrm) > 0.0:   # counter-clockwise from outside: turn it
				var tmp := bb
				bb = cc
				cc = tmp
			for v in [a, bb, cc]:
				verts.append(v)
				normals.append(nrm)
				tints.append(colour)
				uvs.append(uv)


static func far_ring(r: Array) -> PackedVector2Array:
	## A ring's eight corners (asset frame), counter-clockwise from the south side's west end:
	## its box [x0, y0, x1, y1] cut by its diagonal extents [u0, u1, v0, v1] (u = x + y,
	## v = x - y). A corner a slab only touches repeats its neighbour, so every ring has eight
	## and two rings join corner to corner.
	var x0: float = r[0]
	var y0: float = r[1]
	var x1: float = r[2]
	var y1: float = r[3]
	var u0: float = r[4]
	var u1: float = r[5]
	var v0: float = r[6]
	var v1: float = r[7]
	return PackedVector2Array([
		Vector2(clampf(u0 - y0, x0, x1), y0), Vector2(clampf(v1 + y0, x0, x1), y0),
		Vector2(x1, clampf(x1 - v1, y0, y1)), Vector2(x1, clampf(u1 - x1, y0, y1)),
		Vector2(clampf(u1 - y1, x0, x1), y1), Vector2(clampf(v0 + y1, x0, x1), y1),
		Vector2(x0, clampf(x0 - v0, y0, y1)), Vector2(x0, clampf(u0 - x0, y0, y1)),
	])


static func far_outline(r: Array) -> PackedVector2Array:
	## A ring's outline with its repeated corners once: a box's is its four corners.
	var kept := PackedVector2Array()
	for p in far_ring(r):
		if kept.is_empty() or kept[kept.size() - 1].distance_to(p) > 1e-3:
			kept.append(p)
	if kept.size() > 1 and kept[0].distance_to(kept[kept.size() - 1]) <= 1e-3:
		kept.remove_at(kept.size() - 1)
	return kept


static func _far_frustum(verts: PackedVector3Array, normals: PackedVector3Array, tints: PackedColorArray,
		uvs: PackedVector2Array, t: Transform3D, part: Array, colour: Color, uv: Vector2) -> void:
	## A band from its bottom ring at its bounds' z0 to its top ring at z1, placed by t: its sides
	## and its top, wound clockwise seen from outside (Godot's front faces). A band whose two rings
	## are one box is drawn as _far_box draws it, flat; a round band's sides are shaded round
	## (each vertex's normal its level direction from the band's axis), its top flat.
	var b: Array = part[0]
	if part.size() < 5:
		_far_box(verts, normals, tints, uvs, t, b, colour, uv)
		return
	var bottom := far_ring(part[3])
	var top_r := far_ring(part[4])
	if _same_ring(part[3], part[4]) and far_outline(part[3]).size() == 4:
		_far_box(verts, normals, tints, uvs, t, [part[3][0], part[3][1], b[2], part[3][2], part[3][3], b[5]], colour, uv)
		return
	var round := far_outline(part[3]).size() > 4 or far_outline(part[4]).size() > 4
	var z0: float = b[2]
	var z1: float = b[5]
	var centre: Vector3 = t * g((float(b[0]) + float(b[3])) / 2.0, (float(b[1]) + float(b[4])) / 2.0, (z0 + z1) / 2.0)
	var up := (t.basis * Vector3.UP).normalized()
	var tris := []   # [a, b, c, outward normal, round]
	for i in 8:
		var j := (i + 1) % 8
		var p0 := t * g(bottom[i].x, bottom[i].y, z0)
		var p1 := t * g(bottom[j].x, bottom[j].y, z0)
		var p2 := t * g(top_r[j].x, top_r[j].y, z1)
		var p3 := t * g(top_r[i].x, top_r[i].y, z1)
		for tri in [[p0, p1, p2], [p0, p2, p3]]:
			tris.append([tri[0], tri[1], tri[2], true])
	var cap: Array[Vector3] = []
	for p in far_outline(part[4]):
		cap.append(t * g(p.x, p.y, z1))
	for i in range(1, cap.size() - 1):
		tris.append([cap[0], cap[i], cap[i + 1], false])
	for tri in tris:
		var a: Vector3 = tri[0]
		var bb: Vector3 = tri[1]
		var cc: Vector3 = tri[2]
		var cr := (bb - a).cross(cc - a)
		if cr.length() < 1e-8:
			continue   # a side between two corners a slab only touched
		var nrm := cr.normalized()
		if nrm.dot((a + bb + cc) / 3.0 - centre) < 0.0:
			nrm = -nrm
		if cr.dot(nrm) > 0.0:   # counter-clockwise from outside: turn it
			var tmp := bb
			bb = cc
			cc = tmp
		for v in [a, bb, cc]:
			verts.append(v)
			var vn := nrm
			if tri[3] and round:
				var rr: Vector3 = v - centre
				rr -= up * rr.dot(up)
				if rr.length() > 1e-6:
					vn = rr.normalized()
			normals.append(vn)
			tints.append(colour)
			uvs.append(uv)


static func _far_steps(a: int, b: int, quads: int) -> Array[int]:
	## Posts a to b, every ceil((b - a) / quads)-th, b always kept.
	var step := maxi(1, ceili(float(b - a) / float(maxi(quads, 1))))
	var out: Array[int] = []
	var i := a
	while i < b:
		out.append(i)
		i += step
	out.append(b)
	return out


func chunk_rect(n: int) -> Rect2:
	## Chunk n's `bounds` (build.md 5, M6): its first and last posts in plan metres; from its
	## `posts` in a build whose chunks hold no `bounds`.
	var b = _manifest.chunks[n].get("bounds")
	if b is Array and b.size() == 4:
		return Rect2(float(b[0]), float(b[1]), float(b[2]) - float(b[0]), float(b[3]) - float(b[1]))
	var p: Array = _manifest.chunks[n].get("posts", [0, 0, 0, 0])
	var a := Vector2(_x0 + int(p[0]) * _res, _y0 + int(p[1]) * _res)
	return Rect2(a, Vector2(_x0 + int(p[2]) * _res, _y0 + int(p[3]) * _res) - a)


func frame() -> Dictionary:
	return _manifest.get("frame", {})


func file_digest(file: String) -> String:
	## A build file's digest from the manifest's `files` (build.md 4), or "" when it is not listed.
	var e = _manifest.get("files", {}).get(file)
	return String(e.get("digest", "")) if e is Dictionary else ""


func mesh_list() -> Array:
	var list = _manifest.get("meshes")
	return list if list is Array else []


func make_mesh(e) -> MeshInstance3D:
	## One generated mesh as `make` puts it under `meshes` (with its body); null when it is bad.
	return _mesh_node(e, _manifest.get("materials", {}))


func mesh_rect(e) -> Rect2:
	## A generated mesh's plan bounds from its .glb's JSON alone (build.md 10: one node's
	## translation plus the POSITION accessor's min and max, glTF (X, Y, Z) = (x, z, -y)), so a
	## streamer knows the chunks it meets without reading its triangles. A negative size when the
	## file cannot say (stream.gd then keeps it with every chunk).
	var none := Rect2(0, 0, -1, -1)
	if not e is Dictionary or not e.get("file") is String:
		return none
	var f := FileAccess.open(_dir + "/" + String(e.file), FileAccess.READ)
	if f == null or f.get_length() < 20 or f.get_32() != 0x46546C67:
		return none
	f.get_32()
	f.get_32()
	var n := f.get_32()
	if f.get_32() != 0x4E4F534A:
		return none
	var doc = JSON.parse_string(f.get_buffer(n).get_string_from_utf8())
	if not doc is Dictionary:
		return none
	for node in doc.get("nodes", []):
		if not node.has("mesh"):
			continue
		var t: Array = node.get("translation", [0, 0, 0])
		var prim: Dictionary = doc.meshes[int(node.mesh)].primitives[0]
		var acc: Dictionary = doc.accessors[int(prim.attributes.POSITION)]
		var lo: Array = acc.get("min", [])
		var hi: Array = acc.get("max", [])
		if lo.size() != 3 or hi.size() != 3:
			return none
		var a := Vector2(float(t[0]) + float(lo[0]), -(float(t[2]) + float(hi[2])))
		var b := Vector2(float(t[0]) + float(hi[0]), -(float(t[2]) + float(lo[2])))
		return Rect2(a, b - a)
	return none


func _read_manifest() -> bool:
	var path := _dir + "/manifest.json"
	if not FileAccess.file_exists(path):
		var named := "diagnostics.json" if FileAccess.file_exists(_dir + "/diagnostics.json") else "manifest.json"
		diags.add("load.format", "world", named,
			"no manifest.json in %s: the build has an error or is not a build" % _dir.get_file(),
			"read diagnostics.json beside it, fix the world and build again", {})
		return false
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not parsed is Dictionary or parsed.get("format") != FORMAT:
		var fmt = parsed.get("format") if parsed is Dictionary else null
		diags.add("load.format", "world", "manifest.json",
			"manifest format %s; this loader reads %s" % [JSON.stringify(fmt), FORMAT],
			"build again with a via release that writes %s, or use a loader that reads this format" % FORMAT,
			{"format": fmt})
		return false
	_manifest = parsed
	var frame: Dictionary = _manifest.get("frame", {})
	var ok: bool = frame.get("posts") is Array and frame.posts.size() == 2 and frame.get("extent") is Array \
		and frame.extent.size() == 4 and (frame.get("resolution") is float or frame.get("resolution") is int)
	if not ok:
		diags.add("load.bad_file", "world", "manifest.json", "the manifest's frame lacks posts, extent or resolution",
			"build again; a manifest is never edited by hand", {"key": "frame"})
		return false
	_nx = int(frame.posts[0])
	_ny = int(frame.posts[1])
	_x0 = float(frame.extent[0])
	_y0 = float(frame.extent[1])
	_res = float(frame.resolution)
	return true


func _bad_file(file: String, why: String) -> void:
	## A pack's file lies outside the build: the hint says where the loader looked, not to rebuild
	## (M7 close: a pack_root one folder off read as a damaged build).
	var hint := "build again; the directory is replaced whole, so a damaged file means it was changed by hand"
	if file.begins_with("pack:"):
		hint = "the pack's file is read from the manifest's locations.pack_root (or --pack DIR when it is null): check the path above, and that the pack's files are where the build said"
	diags.add("load.bad_file", "world", file, "%s: %s" % [file, why], hint, {"reason": why})


func _field_with_role(fields: Dictionary, role: String, type: String) -> String:
	var names := fields.keys()
	names.sort()
	for n in names:
		if fields[n].get("role") == role and fields[n].get("type") == type:
			return n
	return ""


func _select(fields: Dictionary, selector: String) -> Array:
	var out: Array = []
	for n in fields:
		var e: Dictionary = fields[n]
		var tags: Array = e.get("tags") if e.get("tags") is Array else []
		if (selector == n or (selector.begins_with("role:") and e.get("role") == selector.trim_prefix("role:"))
				or (selector.begins_with("tag:") and selector.trim_prefix("tag:") in tags)):
			out.append(n)
	out.sort()
	return out


func _scalar(name: String, entry: Dictionary) -> PackedFloat32Array:
	var file := String(entry.get("file", ""))
	if entry.has("cover"):
		return PackedFloat32Array()   # a bounded field (M3) is never drawn (build.md 15.1)
	var arr := Npy.read(_dir + "/" + file)
	var why := Npy.expect(arr, "<f4", [_ny, _nx])
	if why != "":
		_bad_file(file, why)
		return PackedFloat32Array()
	return Npy.floats(arr)


func _cut_of(entry: Dictionary) -> PackedByteArray:
	## The terrain entry's `cut` file (build.md 6.5), `|u1` of shape (ny - 1, nx - 1). No `cut` key
	## (a build from before M5) cuts nothing; a key whose file is bad is load.bad_file and cuts
	## nothing, so the terrain is drawn whole rather than with holes guessed.
	var file = entry.get("cut")
	if not file is String:
		return PackedByteArray()
	var arr := Npy.read(_dir + "/" + file)
	var why := Npy.expect(arr, "|u1", [_ny - 1, _nx - 1])
	if why != "":
		_bad_file(file, why)
		return PackedByteArray()
	return arr.bytes


func is_cut(i: int, j: int) -> bool:
	## Whether the quad whose south-west post is [i, j] is cut (build.md 6.5).
	return not _cut.is_empty() and i >= 0 and j >= 0 and i < _nx - 1 and j < _ny - 1 and _cut[j * (_nx - 1) + i] == 1


func cut_at(x: float, y: float) -> bool:
	## Whether the plan point lies on a cut quad.
	return is_cut(floori((x - _x0) / _res), floori((y - _y0) / _res))


func _h(i: int, j: int) -> float:
	return _height[clampi(j, 0, _ny - 1) * _nx + clampi(i, 0, _nx - 1)]


func _chunk(chunk: Dictionary, material: String, cols: Dictionary, with_water: bool) -> Node3D:
	var at: Array = chunk.get("at", [0, 0])
	var node := Node3D.new()
	node.name = "c_%d_%d" % [int(at[0]), int(at[1])]
	var p: Array = chunk.get("posts", [0, 0, 0, 0])
	var i0 := int(p[0])
	var j0 := int(p[1])
	var i1 := int(p[2])
	var j1 := int(p[3])
	if not _height.is_empty():
		var ground := MeshInstance3D.new()
		ground.name = "ground"
		ground.mesh = _ground_mesh(i0, j0, i1, j1)
		var w: Dictionary = chunk.get("weights", {}).get(material, {}) if material != "" else {}
		ground.material_override = _ground_material(node.name, w)
		node.add_child(ground)
		node.add_child(_ground_body(i0, j0, i1, j1))
		if with_water:
			var water := _water_mesh(i0, j0, i1, j1)
			if water != null:
				node.add_child(water)
	var groups: Dictionary = chunk.get("instances", {})
	var assets := groups.keys()
	assets.sort()
	for asset in assets:
		var r: Array = groups[asset]
		node.add_child(_multimesh(asset, int(r[0]), int(r[1]), cols))
		_bodies(node, asset, int(r[0]), int(r[1]), cols)
	return node


func _ground_mesh(i0: int, j0: int, i1: int, j1: int) -> ArrayMesh:
	## The chunk's posts as a grid; each quad split from its south-west to its north-east post.
	var cols := i1 - i0 + 1
	var rows := j1 - j0 + 1
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	verts.resize(cols * rows)
	normals.resize(cols * rows)
	uvs.resize(cols * rows)
	for j in range(j0, j1 + 1):
		for i in range(i0, i1 + 1):
			var n := (j - j0) * cols + (i - i0)
			verts[n] = g(_x0 + i * _res, _y0 + j * _res, _h(i, j))
			var dx := (_h(i + 1, j) - _h(i - 1, j)) / ((mini(i + 1, _nx - 1) - maxi(i - 1, 0)) * _res)
			var dy := (_h(i, j + 1) - _h(i, j - 1)) / ((mini(j + 1, _ny - 1) - maxi(j - 1, 0)) * _res)
			normals[n] = g(-dx, -dy, 1.0).normalized()
			uvs[n] = Vector2((i - i0 + 0.5) / cols, (j - j0 + 0.5) / rows)
	var mesh := ArrayMesh.new()
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	# A cut quad (build.md 6.5) is not drawn; the hole's sides are the library's instances.
	var idx := _grid_indices(cols, rows, func(c, r): return not is_cut(i0 + c, j0 + r))
	arrays[Mesh.ARRAY_INDEX] = idx
	mesh.set_meta("triangles", idx.size() / 3)
	if not idx.is_empty():
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


static func _grid_indices(cols: int, rows: int, keep: Callable) -> PackedInt32Array:
	## Two triangles per kept quad, clockwise seen from above (Godot's front faces):
	## sw, ne, se and sw, nw, ne.
	var idx := PackedInt32Array()
	for r in rows - 1:
		for c in cols - 1:
			if not keep.call(c, r):
				continue
			var sw := r * cols + c
			var se := sw + 1
			var nw := sw + cols
			var ne := nw + 1
			idx.append_array([sw, ne, se, sw, nw, ne])
	return idx


func _ground_body(i0: int, j0: int, i1: int, j1: int) -> StaticBody3D:
	## HeightMapShape3D spaces samples 1 unit apart and centres them; heights are divided by the
	## resolution and the node scaled by it. Its rows run along +Z, which is south. A chunk with a
	## cut quad (build.md 6.5) holds a ConcavePolygonShape3D of its uncut quads instead, split as the
	## mesh is: a HeightMapShape3D has no holes in Godot's physics (a NaN post is not one).
	var cols := i1 - i0 + 1
	var rows := j1 - j0 + 1
	var any_cut := false
	for j in range(j0, j1):
		for i in range(i0, i1):
			any_cut = any_cut or is_cut(i, j)
	if any_cut:
		return _cut_ground_body(i0, j0, i1, j1)
	var data := PackedFloat32Array()
	data.resize(cols * rows)
	for r in rows:
		for c in cols:
			data[r * cols + c] = _h(i0 + c, j1 - r) / _res
	var shape := HeightMapShape3D.new()
	shape.map_width = cols
	shape.map_depth = rows
	shape.map_data = data
	var body := StaticBody3D.new()
	body.name = "ground_body"
	var cs := CollisionShape3D.new()
	cs.name = "shape"
	cs.shape = shape
	cs.position = g(_x0 + (i0 + i1) * 0.5 * _res, _y0 + (j0 + j1) * 0.5 * _res, 0.0)
	cs.scale = Vector3.ONE * _res
	body.add_child(cs)
	return body


func _cut_ground_body(i0: int, j0: int, i1: int, j1: int) -> StaticBody3D:
	## The chunk's uncut quads as triangles (sw, ne, se and sw, nw, ne), collided from both sides.
	var faces := PackedVector3Array()
	for j in range(j0, j1):
		for i in range(i0, i1):
			if is_cut(i, j):
				continue
			var sw := g(_x0 + i * _res, _y0 + j * _res, _h(i, j))
			var se := g(_x0 + (i + 1) * _res, _y0 + j * _res, _h(i + 1, j))
			var nw := g(_x0 + i * _res, _y0 + (j + 1) * _res, _h(i, j + 1))
			var ne := g(_x0 + (i + 1) * _res, _y0 + (j + 1) * _res, _h(i + 1, j + 1))
			faces.append_array([sw, ne, se, sw, nw, ne])
	var shape := ConcavePolygonShape3D.new()
	shape.backface_collision = true
	shape.set_faces(faces)
	var body := StaticBody3D.new()
	body.name = "ground_body"
	var cs := CollisionShape3D.new()
	cs.name = "shape"
	cs.shape = shape
	body.add_child(cs)
	return body


func _ground_material(chunk_name: String, w: Dictionary) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = TERRAIN_SHADER
	mat.set_shader_parameter("default_colour", DEFAULT_GROUND.srgb_to_linear())
	mat.set_shader_parameter("count", 0)
	var file = w.get("file")
	var names: Array = w.get("materials", [])
	if file == null or names.is_empty():
		return mat
	var arr := Npy.read(_dir + "/" + String(file))
	var p: Array = _chunk_posts_of(file)
	var why: String = Npy.expect(arr, "|u1", [names.size(), p[1], p[0]]) if p.size() == 2 else "no chunk lists this file"
	if why != "":
		_bad_file(String(file), why)
		return mat
	var drawn := names.size()
	if drawn > MAX_MATERIALS:
		drawn = MAX_MATERIALS
		diags.add("load.too_many_materials", "chunk." + chunk_name, String(file),
			"chunk %s names %d materials; this loader draws the first %d" % [chunk_name, names.size(), MAX_MATERIALS],
			"paint fewer materials in this chunk, or use a loader that draws more",
			{"chunk": chunk_name, "drawn": MAX_MATERIALS, "materials": names})
	var cols: int = p[0]
	var rows: int = p[1]
	var plane := cols * rows
	var layers: Array[Image] = []
	for m in drawn:
		layers.append(Image.create_from_data(cols, rows, false, Image.FORMAT_R8, arr.bytes.slice(m * plane, (m + 1) * plane)))
	var tex := Texture2DArray.new()
	tex.create_from_images(layers)
	var params := Image.create_empty(drawn, 2, false, Image.FORMAT_RGBAF)
	var mats: Dictionary = _manifest.get("materials", {})
	var any_file := false
	var albedo: Array[Image] = []
	for m in drawn:
		var e: Dictionary = mats.get(names[m], {})
		var c: Array = e.get("colour", [0.8, 0.8, 0.8])
		var img := _material_image(names[m], e)
		any_file = any_file or img != null
		# A pack colour is linear already (assets.md 2.6), as an asset's is: drawn as declared.
		params.set_pixel(m, 0, Color(c[0], c[1], c[2], 1.0 if img != null else 0.0))
		params.set_pixel(m, 1, Color(float(e.get("repeat", 1.0)), 0, 0, 1))
		albedo.append(img)
	mat.set_shader_parameter("weights", tex)
	mat.set_shader_parameter("params", ImageTexture.create_from_image(params))
	mat.set_shader_parameter("count", drawn)
	if any_file:
		var white := Image.create_empty(ALBEDO_SIZE, ALBEDO_SIZE, false, Image.FORMAT_RGBA8)
		white.fill(Color.WHITE)
		white.generate_mipmaps()
		var full: Array[Image] = []
		for img in albedo:
			full.append(img if img != null else white)
		var at := Texture2DArray.new()
		at.create_from_images(full)
		mat.set_shader_parameter("albedo", at)
	return mat


func _chunk_posts_of(file) -> Array:
	## [columns, rows] of the chunk whose weights file this is.
	for chunk in _manifest.get("chunks", []):
		for f in chunk.get("weights", {}).values():
			if f.get("file") == file:
				var p: Array = chunk.posts
				return [int(p[2]) - int(p[0]) + 1, int(p[3]) - int(p[1]) + 1]
	return []


func _pack_root() -> String:
	var pr = _manifest.get("locations", {}).get("pack_root")
	if pr is String:
		return (_dir + "/" + pr).simplify_path()
	return pack_dir


func _asset_root(a: Dictionary) -> String:
	## Where an asset's files lie (build.md 4, 7): an asset an import's pack binds names that
	## import in `pack`, and its root is `locations.pack_roots`; any other is under `pack_root`.
	if a.get("pack") is String:
		var pr = _manifest.get("locations", {}).get("pack_roots", {}).get(String(a.pack))
		return (_dir + "/" + pr).simplify_path() if pr is String else ""
	return _pack_root()


func _material_image(id: String, e: Dictionary) -> Image:
	## A material's `file`, read from the pack's root, resampled for the shared array; null when it
	## has none, or it cannot be read (then load.bad_file and its colour is drawn).
	if not e.has("file") or not e.file is String:
		return null
	if _albedo.has(id):
		return _albedo[id]
	var img: Image = null
	var root := _pack_root()
	var path := root + "/" + String(e.file)
	if root == "" or not FileAccess.file_exists(path):
		_bad_file("pack:" + String(e.file), ("missing at %s" % path) if root != "" else "the manifest's pack_root is null and no pack directory was given")
	else:
		img = Image.load_from_file(path)
		if img == null or img.is_empty():
			_bad_file("pack:" + String(e.file), "not an image Godot reads")
			img = null
		else:
			img.convert(Image.FORMAT_RGBA8)
			img.resize(ALBEDO_SIZE, ALBEDO_SIZE, Image.INTERPOLATE_CUBIC)
			img.generate_mipmaps()
	_albedo[id] = img
	return img


func _water_mesh(i0: int, j0: int, i1: int, j1: int) -> MeshInstance3D:
	## The water surface over the posts where depth (water - ground) is above 0; a quad is kept
	## when any of its posts is wet, so the surface meets the bank where it is dry. Not collided.
	var cols := i1 - i0 + 1
	var rows := j1 - j0 + 1
	var wet := PackedByteArray()
	wet.resize(cols * rows)
	var any := false
	for r in rows:
		for c in cols:
			var k := (j0 + r) * _nx + (i0 + c)
			if _water[k] > _height[k]:
				wet[r * cols + c] = 1
				any = true
	if not any:
		return null
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var colours := PackedColorArray()
	verts.resize(cols * rows)
	normals.resize(cols * rows)
	colours.resize(cols * rows)
	for r in rows:
		for c in cols:
			var k := (j0 + r) * _nx + i0 + c
			var z := _water[k]
			if wet[r * cols + c] == 0:
				z = _shore_level(i0 + c, j0 + r, z)
			verts[r * cols + c] = g(_x0 + (i0 + c) * _res, _y0 + (j0 + r) * _res, z)
			normals[r * cols + c] = Vector3.UP
			# Shallow water lets the bed show through; the edge, where depth is 0, most of all.
			var a := clampf(WATER_CLEAR + (z - _height[k]) * WATER_THICKEN, WATER_CLEAR, WATER_COLOUR.a)
			colours[r * cols + c] = Color(1, 1, 1, a)
	var keep := func(c, r):
		var n: int = r * cols + c
		return wet[n] == 1 or wet[n + 1] == 1 or wet[n + cols] == 1 or wet[n + cols + 1] == 1
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colours
	arrays[Mesh.ARRAY_INDEX] = _grid_indices(cols, rows, keep)
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mi := MeshInstance3D.new()
	mi.name = "water"
	mi.mesh = mesh
	mi.material_override = water_material()
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return mi


func _shore_level(i: int, j: int, ground: float) -> float:
	## A dry post at the water's edge, always strictly below its own ground, so the water's edge
	## crosses each quad edge from a wet post to it between the two posts, never on the post
	## (build.md 15.1). Where the mean height of its wet neighbours (the eight around it, in the
	## whole field, so chunks agree at their seams) lies at least SHORE_GAP below its ground, the
	## surface runs on level under the bank at that mean, and the edge is where the bank meets it.
	## Otherwise (a falling stream, whose upstream neighbours stand above this post's ground) it
	## takes its ground less the wet neighbours' mean depth, at least SHORE_GAP: depth runs from
	## +d at the wet post to -d here, so the edge falls about half way between them. A post with
	## no wet neighbour keeps its own height.
	var sum := 0.0
	var deep := 0.0
	var n := 0
	for dj in [-1, 0, 1]:
		for di in [-1, 0, 1]:
			var a: int = i + di
			var b: int = j + dj
			if (di == 0 and dj == 0) or a < 0 or b < 0 or a >= _nx or b >= _ny:
				continue
			var k := b * _nx + a
			if _water[k] > _height[k]:
				sum += _water[k]
				deep += _water[k] - _height[k]
				n += 1
	if n == 0:
		return ground
	var level := sum / n
	if level <= ground - SHORE_GAP:
		return level
	return ground - maxf(deep / n, SHORE_GAP)


func water_material() -> StandardMaterial3D:
	if _water_material == null:
		_water_material = StandardMaterial3D.new()
		_water_material.albedo_color = Color(WATER_COLOUR, 1.0)
		_water_material.vertex_color_use_as_albedo = true   # alpha by depth, per vertex
		_water_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		_water_material.roughness = 0.25
		_water_material.metallic_specular = 0.3
		# Seen from below too: an eye under the surface sees water, never an empty pit.
		_water_material.cull_mode = BaseMaterial3D.CULL_DISABLED
	return _water_material


# --- instances -----------------------------------------------------------------------------------

func _instance_columns() -> Dictionary:
	## Every column, checked against the manifest's count; an empty dictionary when any is bad.
	var inst: Dictionary = _manifest.get("instances", {})
	var count := int(inst.get("count", 0))
	var out := {"count": count}
	var want := {"x": "<i4", "y": "<i4", "z": "<f4", "face": "<i4", "scale": "<f4", "tint": "<f4"}
	for col in want:
		var file := String(inst.get(col, "instances/%s.npy" % col))
		var arr := Npy.read(_dir + "/" + file)
		var why := Npy.expect(arr, want[col], [count])
		if why != "":
			_bad_file(file, why)
			return {}
		out[col] = Npy.ints(arr) if want[col] == "<i4" else Npy.floats(arr)
	var names_file := String(inst.get("names", "instances/names.txt"))
	if not FileAccess.file_exists(_dir + "/" + names_file):
		_bad_file(names_file, "missing")
		return {}
	var names := FileAccess.get_file_as_string(_dir + "/" + names_file).split("\n")
	if names.size() > 0 and names[-1] == "":
		names.resize(names.size() - 1)
	if names.size() != count:
		_bad_file(names_file, "%d names for %d instances" % [names.size(), count])
		return {}
	out["names"] = names
	return out


func _instance_transform(cols: Dictionary, n: int, with_scale: bool) -> Transform3D:
	## T(x, y, z) R(face) S(scale) of build.md 8; a bearing b is a turn of -b about +Y.
	var b := float(cols.face[n]) / 1000.0
	var basis := Basis(Vector3.UP, deg_to_rad(-b))
	if with_scale:
		basis = basis.scaled(Vector3.ONE * cols.scale[n])
	return Transform3D(basis, g(cols.x[n] / 1000.0, cols.y[n] / 1000.0, cols.z[n]))


func _asset_mesh_of(id: String) -> Mesh:
	if _asset_mesh.has(id):
		return _asset_mesh[id]
	var a: Dictionary = _manifest.get("assets", {}).get(id, {})
	var mesh: Mesh = null
	var bounds: Array = a.get("bounds", [])
	if a.get("stand_in", false) and bounds.size() == 6:
		mesh = Primitives.mesh("box", bounds)       # a stand-in is a box of its bounds (assets.md 9)
	elif a.get("source") == "primitive" and bounds.size() == 6:
		mesh = Primitives.mesh(String(a.get("primitive", "")), bounds)
	elif a.get("source") == "model":
		mesh = _model_mesh(a)
		_asset_mesh[id] = mesh
		return mesh   # drawn with the model's own materials
	# An engine-made asset has no mesh here; its rows are still held.
	_asset_mesh[id] = mesh
	if mesh != null:
		var mat := ShaderMaterial.new()
		mat.shader = INSTANCE_SHADER
		var c: Array = a.get("colour", [0.8, 0.8, 0.8])
		mat.set_shader_parameter("colour", Color(c[0], c[1], c[2]))
		_asset_material[id] = mat
	return mesh


func _model_mesh(a: Dictionary) -> Mesh:
	## A model asset's mesh (build.md 7): its first file read as glTF from the pack's root, every
	## surface's vertices taken by `to_asset` from the file's node space to the asset frame, then
	## to Godot's axes. Null, with load.bad_file, when the file is missing or unreadable.
	var files: Array = a.get("files", [])
	if files.is_empty() or not files[0] is Dictionary:
		return null
	var rel := String(files[0].get("path", ""))
	var root := _asset_root(a)
	var path := root + "/" + rel
	if root == "" or not FileAccess.file_exists(path):
		_bad_file("pack:" + rel, ("missing at %s" % path) if root != "" else "the manifest names no root for this asset's pack, and no pack directory was given")
		return null
	var doc := GLTFDocument.new()
	var state := GLTFState.new()
	if doc.append_from_file(path, state) != OK:
		_bad_file("pack:" + rel, "not glTF Godot reads")
		return null
	var scene: Node = doc.generate_scene(state)
	if scene == null:
		_bad_file("pack:" + rel, "no scene")
		return null
	var m: Array = a.get("to_asset", [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0])
	var out := ArrayMesh.new()
	_add_model_surfaces(scene, Transform3D.IDENTITY, m, out)
	scene.free()
	return out if out.get_surface_count() > 0 else null


static func _to_godot(m: Array, v: Vector3, translate: bool) -> Vector3:
	## `to_asset` (3 by 4, row by row) on a point (or a direction), then the world frame to Godot.
	var t := 1.0 if translate else 0.0
	var x: float = m[0] * v.x + m[1] * v.y + m[2] * v.z + m[3] * t
	var y: float = m[4] * v.x + m[5] * v.y + m[6] * v.z + m[7] * t
	var z: float = m[8] * v.x + m[9] * v.y + m[10] * v.z + m[11] * t
	return g(x, y, z)


func _add_model_surfaces(node: Node, parent: Transform3D, m: Array, out: ArrayMesh) -> void:
	var xf := parent
	if node is Node3D:
		xf = parent * (node as Node3D).transform
	if node is MeshInstance3D and (node as MeshInstance3D).mesh != null:
		var src: Mesh = (node as MeshInstance3D).mesh
		for si in src.get_surface_count():
			var arrays := src.surface_get_arrays(si)
			var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			for k in verts.size():
				verts[k] = _to_godot(m, xf * verts[k], true)
			arrays[Mesh.ARRAY_VERTEX] = verts
			if arrays[Mesh.ARRAY_NORMAL] is PackedVector3Array:
				var nrm: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
				for k in nrm.size():
					nrm[k] = _to_godot(m, xf.basis * nrm[k], false).normalized()
				arrays[Mesh.ARRAY_NORMAL] = nrm
			arrays[Mesh.ARRAY_TANGENT] = null
			# A turn of axes may flip the winding: keep the faces' fronts outward.
			var det := Basis(Vector3(m[0], m[4], m[8]), Vector3(m[1], m[5], m[9]), Vector3(m[2], m[6], m[10])).determinant()
			if det < 0.0 and arrays[Mesh.ARRAY_INDEX] is PackedInt32Array:
				var idx: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
				for k in range(0, idx.size() - 2, 3):
					var tmp := idx[k + 1]
					idx[k + 1] = idx[k + 2]
					idx[k + 2] = tmp
				arrays[Mesh.ARRAY_INDEX] = idx
			out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
			out.surface_set_material(out.get_surface_count() - 1, src.surface_get_material(si))
	for c in node.get_children():
		_add_model_surfaces(c, xf, m, out)


func _multimesh(asset: String, first: int, count: int, cols: Dictionary) -> MultiMeshInstance3D:
	var mmi := MultiMeshInstance3D.new()
	mmi.name = asset
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = _asset_mesh_of(asset)
	var ok := cols.has("names") and first >= 0 and first + count <= int(cols["count"])
	mm.instance_count = count if ok else 0
	var names := PackedStringArray()
	if ok:
		for k in count:
			var n := first + k
			mm.set_instance_transform(k, _instance_transform(cols, n, true))
			mm.set_instance_custom_data(k, Color(cols.tint[n], 0, 0, 0))
			names.append(cols.names[n])
	mmi.multimesh = mm
	if _asset_material.has(asset):
		mmi.material_override = _asset_material[asset]
	mmi.set_meta("names", names)
	mmi.set_meta("asset", asset)
	return mmi


static func collision_of(a: Dictionary):
	## What an asset collides as: its `collision` fact (assets.md 2.4, M2), else the M1 convention
	## `params.collision`; null for none. `none` and any other word make no body.
	var kind = a.get("collision")
	if kind == null:
		var params = a.get("params", {})
		kind = params.get("collision") if params is Dictionary else null
	return kind if kind in ["box", "hull", "mesh"] else null


func _bodies(node: Node3D, asset: String, first: int, count: int, cols: Dictionary) -> void:
	## Collision from the asset's record (collision_of): box from bounds, hull a convex shape,
	## mesh a concave one, each on a StaticBody3D named after the instance. No collision, no
	## body. The scale is put into the shape, not the body.
	var a: Dictionary = _manifest.get("assets", {}).get(asset, {})
	var kind = collision_of(a)
	if kind == null or not cols.has("names"):
		return
	var bounds: Array = a.get("bounds", [])
	if bounds.size() != 6:
		return
	var p = a.get("primitive")
	var prim := "box" if a.get("stand_in", false) else (String(p) if p is String else "")
	for k in count:
		var n := first + k
		var s: float = cols.scale[n]
		var cs := CollisionShape3D.new()
		cs.name = "shape"
		match kind:
			"box":
				var box := BoxShape3D.new()
				box.size = Vector3(bounds[3] - bounds[0], bounds[5] - bounds[2], bounds[4] - bounds[1]) * s
				cs.shape = box
				cs.position = g((bounds[0] + bounds[3]) / 2.0, (bounds[1] + bounds[4]) / 2.0, (bounds[2] + bounds[5]) / 2.0) * s
			"hull":
				var pts := Primitives.points(prim, bounds, s)
				if pts.is_empty():
					continue
				var hull := ConvexPolygonShape3D.new()
				hull.points = pts
				cs.shape = hull
			"mesh":
				# The faces Primitives.mesh draws, taken from its triangles, not read back from
				# the mesh: a read back waits on the renderer, which a worker thread must not.
				var tris := Primitives.triangles(prim, bounds)
				if tris.is_empty():
					continue
				var faces := PackedVector3Array()
				for tri in tris:
					for v in [tri[0], tri[2], tri[1]]:
						faces.append(Primitives.g(v) * s)
				var con := ConcavePolygonShape3D.new()
				con.set_faces(faces)
				cs.shape = con
		var body := StaticBody3D.new()
		body.name = node_name(cols.names[n])
		body.transform = _instance_transform(cols, n, false)
		body.add_child(cs)
		_bodies_holder(node).add_child(body)


func _bodies_holder(node: Node3D) -> Node3D:
	## The chunk's instance bodies under their own node, `bodies`: beside the multimeshes, which
	## are named after their assets, an instance named like its asset (station's `lens`) was
	## renamed `@StaticBody3D@N` and a walk was `blocked` by that, not by the instance.
	var h := node.get_node_or_null("bodies") as Node3D
	if h == null:
		h = Node3D.new()
		h.name = "bodies"
		node.add_child(h)
	return h


# --- features, bookmarks, meetings -----------------------------------------------------------------

func _features_json() -> Dictionary:
	var file = _manifest.get("features")
	if not file is String:
		return {}
	if not FileAccess.file_exists(_dir + "/" + file):
		_bad_file(file, "missing")
		return {}
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(_dir + "/" + file))
	if not parsed is Dictionary:
		_bad_file(file, "not a JSON object")
		return {}
	return parsed


func _feature_list() -> Array:
	if _features_cache == null:
		_features_cache = _features_json()
	return _features_cache.get("features", [])


static func _vec(p) -> Vector3:
	return g(float(p[0]), float(p[1]), float(p[2]) if p.size() > 2 else 0.0)


func _features(root: Node3D) -> void:
	if not options.features:
		return
	var holder := Node3D.new()
	holder.name = "features"
	root.add_child(holder)
	for f in _feature_list():
		var node: Node3D
		match f.get("shape"):
			"path":
				var path := Path3D.new()
				path.curve = Curve3D.new()
				for p in f.get("points", []):
					path.curve.add_point(_vec(p))
				node = path
			"point":
				node = Marker3D.new()
				node.position = _vec(f.get("at", [0, 0, 0]))
			_:
				node = Node3D.new()
				node.set_meta("polygon", _pieces(f.get("pieces", [])))
		node.name = node_name(String(f.get("name", "feature")))
		for k in ["kind", "authority", "layer"]:
			node.set_meta(k, f.get(k))
		node.set_meta("attrs", f.get("attrs", {}))
		holder.add_child(node)
	var prints := Node3D.new()
	prints.name = "footprints"
	holder.add_child(prints)
	for f in _features_cache.get("footprints", []):
		var node := Node3D.new()
		node.name = node_name(String(f.get("instance", "footprint")))
		node.set_meta("polygon", _pieces(f.get("pieces", [])))
		for k in ["kind", "authority", "layer"]:
			node.set_meta(k, f.get(k))
		node.set_meta("attrs", f.get("attrs", {}))
		prints.add_child(node)


func _paths(root: Node3D) -> bool:
	## The `paths` option: each path feature a selector names (a name, or `kind:<kind>`) as a
	## MeshInstance3D strip under `paths`, named after the feature, draped on the ground: `width`
	## metres across (attrs.width, else PATH_WIDTH), PATH_LIFT above the ground as its mesh draws it
	## (ground_at). Where drawn paths share an end point (a lane ending at a ford and the next lane
	## starting there, geometry.md 2.7), the first of them by name also carries a round join there,
	## a disc as wide as the widest of them, so the bend shows no notch. Not collided. A selector
	## matching no path feature is an option error, as `water` is. False when the option is in error.
	if options.paths.is_empty():
		return true
	var holder := Node3D.new()
	holder.name = "paths"
	root.add_child(holder)
	var paths := _feature_list().filter(func(f): return f.get("shape") == "path" and f.get("points", []).size() >= 2)
	var chosen := {}
	for sel in options.paths:
		var hits: Array = []
		for f in paths:
			if f.get("name") == sel or (sel.begins_with("kind:") and f.get("kind") == sel.trim_prefix("kind:")):
				hits.append(f)
		if hits.is_empty():
			option_error = "paths: `%s` selects no path feature in this build" % sel
			return false
		for f in hits:
			chosen[String(f.name)] = f
	if _height.is_empty():
		return true   # no ground to lay them on: nothing is drawn, as with water
	var names := chosen.keys()
	names.sort()
	var widths := {}
	var ends := {}   # name -> [first point, last point], plan metres
	for n in names:
		var f: Dictionary = chosen[n]
		var attrs: Dictionary = f.get("attrs", {}) if f.get("attrs") is Dictionary else {}
		var w = attrs.get("width")
		widths[n] = float(w) if _finite_number(w) and float(w) > 0.0 else PATH_WIDTH
		var pts: Array = f.points
		ends[n] = [Vector2(float(pts[0][0]), float(pts[0][1])), Vector2(float(pts[-1][0]), float(pts[-1][1]))]
	# Each shared end point once: the first path by name that ends there draws the join.
	var joins := {}   # name -> [[point, radius]]
	var done: Array[Vector2] = []
	for n in names:
		joins[n] = []
		for e in ends[n]:
			if done.any(func(q): return q.distance_to(e) <= JOIN_TOLERANCE):
				continue
			var radius: float = widths[n] * 0.5
			var shared := false
			for m in names:
				if m == n:
					continue
				for o in ends[m]:
					if o.distance_to(e) <= JOIN_TOLERANCE:
						shared = true
						radius = maxf(radius, widths[m] * 0.5)
			if shared:
				joins[n].append([e, radius])
				done.append(e)
	for n in names:
		var f: Dictionary = chosen[n]
		var mi := MeshInstance3D.new()
		mi.name = node_name(n)
		mi.mesh = _strip(f.points, widths[n], joins[n])
		mi.set_meta("across", across_count(widths[n], _res))
		mi.set_meta("joins", joins[n].size())
		mi.material_override = path_material()
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		holder.add_child(mi)
	return true


func ground_at(x: float, y: float) -> float:
	## The ground's height at a plan point as the ground mesh draws it, clamped to the extent: on
	## the triangle of its quad that holds the point, each quad split from its south-west to its
	## north-east post (_ground_mesh). Not bilinear: a strip draped on the bilinear surface sinks
	## under the mesh wherever the quad's diagonal is a ridge.
	return _on_mesh(_height, x, y)


func has_ground() -> bool:
	return not _height.is_empty()


func ground_min() -> float:
	## The lowest post of the ground; INF without one.
	var m := INF
	for h in _height:
		m = minf(m, h)
	return m


func depth_at(x: float, y: float) -> float:
	## Water above the ground at a plan point, read as the meshes draw both (ground_at); 0 where
	## dry, and 0 when no water is drawn.
	if _water.is_empty() or _height.is_empty():
		return 0.0
	return maxf(0.0, _on_mesh(_water, x, y) - _on_mesh(_height, x, y))


func _on_mesh(a: PackedFloat32Array, x: float, y: float) -> float:
	var fx := clampf((x - _x0) / _res, 0.0, _nx - 1.0)
	var fy := clampf((y - _y0) / _res, 0.0, _ny - 1.0)
	var i := mini(int(fx), _nx - 2)
	var j := mini(int(fy), _ny - 2)
	var ax := fx - i
	var ay := fy - j
	var at := func(p: int, q: int) -> float: return a[clampi(q, 0, _ny - 1) * _nx + clampi(p, 0, _nx - 1)]
	var sw: float = at.call(i, j)
	var ne: float = at.call(i + 1, j + 1)
	if ax >= ay:   # sw, se, ne
		var se: float = at.call(i + 1, j)
		return sw + ax * (se - sw) + ay * (ne - se)
	var nw: float = at.call(i, j + 1)
	return sw + ay * (nw - sw) + ax * (ne - nw)   # sw, nw, ne


func solid_without_body() -> Array:
	## Each instance of a `solid` asset (assets.md 2.4) that collides as nothing: { "name", "plan" },
	## its footprint in plan metres, placed by its transform (build.md 8). The walker reports one
	## it walks through, since the core keeps things off it while the engine lets a body pass.
	var out: Array = []
	var cols := _instance_columns()
	if not cols.has("names"):
		return out
	var assets: Dictionary = _manifest.get("assets", {})
	for chunk in _manifest.get("chunks", []):
		var groups: Dictionary = chunk.get("instances", {})
		for asset in groups:
			var a: Dictionary = assets.get(asset, {})
			if a.get("solidity", "solid") != "solid" or collision_of(a) != null:
				continue
			var fp: Array = a.get("footprint", [])
			if fp.size() < 3:
				continue
			var r: Array = groups[asset]
			for k in int(r[1]):
				var n := int(r[0]) + k
				var b := deg_to_rad(float(cols.face[n]) / 1000.0)   # clockwise from north
				var s: float = cols.scale[n]
				var o := Vector2(cols.x[n] / 1000.0, cols.y[n] / 1000.0)
				var plan := PackedVector2Array()
				for p in fp:
					var v := Vector2(float(p[0]), float(p[1])) * s
					plan.append(o + Vector2(v.x * cos(b) + v.y * sin(b), -v.x * sin(b) + v.y * cos(b)))
				out.append({"name": cols.names[n], "plan": plan})
	return out


static func across_count(width: float, res: float) -> int:
	## Vertices across a strip `width` metres wide: no further apart than the post spacing `res`,
	## and an odd count, so one runs down the path's line.
	var gaps := maxi(2, ceili(width / res - 1e-6))
	return gaps + gaps % 2 + 1


func _strip(points: Array, width: float, joins: Array = []) -> ArrayMesh:
	## A strip along the polyline, sampled every half post spacing of arc length and at most one
	## post spacing across (across_count), each vertex on the ground mesh (ground_at), raised by
	## the most the ground mesh rises over any ribbon triangle it is a corner of (_rise_over), plus
	## PATH_LIFT. Across, each cross-section is perpendicular to the line's direction over a window
	## of half the width on either side, so a turn made of short segments (a found course's kink)
	## is spread over the strip's own width and the cross-sections inside it do not cross. A
	## cross-section whose inner end would fall behind the one before keeps the one before's
	## direction. At a vertex where the line turns by more than TURN_DISC degrees a round disc of
	## half the width covers the corner. Each of `joins`, [point, radius], adds a disc there.
	var line: Array[Vector2] = []
	for p in points:
		var q := Vector2(float(p[0]), float(p[1]))
		if line.is_empty() or line[-1] != q:
			line.append(q)
	var cum: Array[float] = [0.0]
	for k in range(1, line.size()):
		cum.append(cum[-1] + (line[k] - line[k - 1]).length())
	var total: float = cum[-1]
	var step := _res * 0.5
	var at: Array[Vector2] = []
	var dirs: Array[Vector2] = []
	var n := maxi(1, ceili(total / step))
	var half := maxf(width * 0.5, step)
	for s in n + 1:
		var u := total * float(s) / n
		at.append(_along(line, cum, u))
		var d := _along(line, cum, minf(total, u + half)) - _along(line, cum, maxf(0.0, u - half))
		if d.length() == 0.0:
			d = (line[-1] - line[0]) if line.size() > 1 else Vector2.RIGHT
		dirs.append(d.normalized() if d.length() > 0.0 else Vector2.RIGHT)
	# Keep each cross-section from passing the one before it on either side.
	for r in range(1, at.size()):
		var prev_side := Vector2(-dirs[r - 1].y, dirs[r - 1].x) * width * 0.5
		var side := Vector2(-dirs[r].y, dirs[r].x) * width * 0.5
		for sg in [-1.0, 1.0]:
			var q0: Vector2 = at[r - 1] + prev_side * sg
			var q1: Vector2 = at[r] + side * sg
			if (q1 - q0).dot(dirs[r - 1]) < 0.0:
				dirs[r] = dirs[r - 1]
				side = prev_side
	var across := across_count(width, _res)
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var plan: Array[Vector2] = []
	var zs := PackedFloat32Array()
	for r in at.size():
		var d := dirs[r]
		var side := Vector2(-d.y, d.x)
		for c in across:
			var q := at[r] + side * width * (float(c) / (across - 1) - 0.5)
			plan.append(q)
			zs.append(ground_at(q.x, q.y))
			normals.append(Vector3.UP)
	var idx := PackedInt32Array()
	for r in at.size() - 1:
		for c in across - 1:
			var a := r * across + c
			var b := a + 1
			var e := a + across
			var f := e + 1
			idx.append_array([a, e, b, b, e, f])
	# The ribbon's vertices are not the ground mesh's posts: where the ground bends between them
	# (a road's camber, a cut's edge) it can stand above a flat ribbon triangle. Each vertex is
	# raised by the most the ground rises over any triangle it is a corner of, then PATH_LIFT.
	var lift := PackedFloat32Array()
	lift.resize(plan.size())
	for t in range(0, idx.size(), 3):
		var k0 := idx[t]
		var k1 := idx[t + 1]
		var k2 := idx[t + 2]
		var rise := _rise_over(plan[k0], plan[k1], plan[k2], zs[k0], zs[k1], zs[k2])
		for k in [k0, k1, k2]:
			lift[k] = maxf(lift[k], rise)
	for k in plan.size():
		verts.append(g(plan[k].x, plan[k].y, zs[k] + lift[k] + PATH_LIFT))
	for k in range(1, line.size() - 1):
		var d0 := (line[k] - line[k - 1]).normalized()
		var d1 := (line[k + 1] - line[k]).normalized()
		if rad_to_deg(absf(d0.angle_to(d1))) > TURN_DISC:
			_disc(verts, normals, idx, line[k], width * 0.5)
	for j in joins:
		_disc(verts, normals, idx, j[0], j[1])
	# A strip or a disc near the world's edge would hang past it, over nothing: what lies past the
	# extent is brought onto its edge.
	verts = _to_extent(verts)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = idx
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _to_extent(verts: PackedVector3Array) -> PackedVector3Array:
	## `verts` with every vertex that lies past the world's extent in plan (Godot X is x, -Z is y)
	## moved onto its edge, at the same height over the ground there. The strip's rows and the
	## discs' rings keep their order, so a strip or disc that reaches past the edge ends along it.
	if _nx < 2 or _ny < 2:
		return verts
	var lo := Vector2(_x0, _y0)
	var hi := Vector2(_x0 + (_nx - 1) * _res, _y0 + (_ny - 1) * _res)
	var out := verts.duplicate()
	for k in out.size():
		var v := out[k]
		var q := Vector2(v.x, -v.z)
		var c := q.clamp(lo, hi)
		if c != q:
			var over := v.y - ground_at(q.x, q.y)
			out[k] = g(c.x, c.y, ground_at(c.x, c.y) + over)
	return out


func _rise_over(p0: Vector2, p1: Vector2, p2: Vector2, z0: float, z1: float, z2: float) -> float:
	## The most the ground mesh stands above the flat triangle (p0, p1, p2) at heights (z0, z1, z2),
	## 0 when it stands nowhere above: sampled at the triangle's centre, at quarters along each
	## edge (where the mesh's quad edges and diagonals cross it), and at every ground post inside
	## it, where a bend of the ground mesh peaks.
	var den := (p1.y - p2.y) * (p0.x - p2.x) + (p2.x - p1.x) * (p0.y - p2.y)
	if absf(den) < 1e-12:
		return 0.0
	var plane := func(q: Vector2) -> float:
		var w0 := ((p1.y - p2.y) * (q.x - p2.x) + (p2.x - p1.x) * (q.y - p2.y)) / den
		var w1 := ((p2.y - p0.y) * (q.x - p2.x) + (p0.x - p2.x) * (q.y - p2.y)) / den
		return w0 * z0 + w1 * z1 + (1.0 - w0 - w1) * z2
	var samples: Array[Vector2] = [(p0 + p1 + p2) / 3.0]
	for e in [[p0, p1], [p1, p2], [p2, p0]]:
		for f in [0.25, 0.5, 0.75]:
			samples.append((e[0] as Vector2).lerp(e[1], f))
	var lo := Vector2(minf(p0.x, minf(p1.x, p2.x)), minf(p0.y, minf(p1.y, p2.y)))
	var hi := Vector2(maxf(p0.x, maxf(p1.x, p2.x)), maxf(p0.y, maxf(p1.y, p2.y)))
	for j in range(ceili((lo.y - _y0) / _res), floori((hi.y - _y0) / _res) + 1):
		for i in range(ceili((lo.x - _x0) / _res), floori((hi.x - _x0) / _res) + 1):
			var q := Vector2(_x0 + i * _res, _y0 + j * _res)
			var w0 := ((p1.y - p2.y) * (q.x - p2.x) + (p2.x - p1.x) * (q.y - p2.y)) / den
			var w1 := ((p2.y - p0.y) * (q.x - p2.x) + (p0.x - p2.x) * (q.y - p2.y)) / den
			if w0 >= 0.0 and w1 >= 0.0 and w0 + w1 <= 1.0:
				samples.append(q)
	var most := 0.0
	for q in samples:
		most = maxf(most, ground_at(q.x, q.y) - plane.call(q))
	return most


static func _along(line: Array[Vector2], cum: Array[float], u: float) -> Vector2:
	## The point at arc length `u` along the polyline (`cum` its running lengths).
	if line.size() == 1:
		return line[0]
	var k := cum.bsearch(u, true)
	k = clampi(k, 1, line.size() - 1)
	var seg: float = cum[k] - cum[k - 1]
	var f := 0.0 if seg == 0.0 else clampf((u - cum[k - 1]) / seg, 0.0, 1.0)
	return line[k - 1].lerp(line[k], f)


func _disc(verts: PackedVector3Array, normals: PackedVector3Array, idx: PackedInt32Array, centre: Vector2, radius: float) -> void:
	## A round join appended to a strip's arrays: rings every half post spacing out to `radius`,
	## JOIN_SEGMENTS around, each vertex on the ground mesh plus PATH_LIFT. Faces up, as the strip's
	## do; the material draws both sides.
	var rings := maxi(1, ceili(radius / (_res * 0.5)))
	var base := verts.size()
	verts.append(g(centre.x, centre.y, ground_at(centre.x, centre.y) + PATH_LIFT))
	normals.append(Vector3.UP)
	for k in range(1, rings + 1):
		var rr := radius * float(k) / rings
		for s in JOIN_SEGMENTS:
			var q := centre + Vector2.from_angle(TAU * float(s) / JOIN_SEGMENTS) * rr
			verts.append(g(q.x, q.y, ground_at(q.x, q.y) + PATH_LIFT))
			normals.append(Vector3.UP)
	for s in JOIN_SEGMENTS:
		var t := (s + 1) % JOIN_SEGMENTS
		idx.append_array([base, base + 1 + s, base + 1 + t])
	for k in range(1, rings):
		var inner := base + 1 + (k - 1) * JOIN_SEGMENTS
		var outer := inner + JOIN_SEGMENTS
		for s in JOIN_SEGMENTS:
			var t := (s + 1) % JOIN_SEGMENTS
			idx.append_array([inner + s, outer + s, outer + t, inner + s, outer + t, inner + t])


var _path_material: ShaderMaterial


func path_material() -> ShaderMaterial:
	## PATH_COLOUR as declared, the way the instance shader draws an asset's colour: a plain linear
	## uniform, made ALBEDO by linear.gdshaderinc so each renderer lights it as declared.
	if _path_material == null:
		var sh := Shader.new()
		sh.code = "shader_type spatial;\nrender_mode cull_disabled;\n#include \"res://addons/via_loader/linear.gdshaderinc\"\nuniform vec3 colour;\nvoid fragment() {\n\tALBEDO = albedo_from_linear(colour);\n\tROUGHNESS = 0.95;\n}\n"
		_path_material = ShaderMaterial.new()
		_path_material.shader = sh
		_path_material.set_shader_parameter("colour", Color(PATH_COLOUR.r, PATH_COLOUR.g, PATH_COLOUR.b))
	return _path_material


static func _pieces(pieces: Array) -> Array:
	## Each piece as { "outer": PackedVector2Array, "holes": [PackedVector2Array] }, plan metres.
	var out: Array = []
	for piece in pieces:
		var ring := func(pts):
			var r := PackedVector2Array()
			for p in pts:
				r.append(Vector2(p[0], p[1]))
			return r
		var holes: Array = []
		for h in piece.get("holes", []):
			holes.append(ring.call(h))
		out.append({"outer": ring.call(piece.get("outer", [])), "holes": holes})
	return out


static func _finite_number(v) -> bool:
	return (v is float or v is int) and is_finite(float(v))


static func _point3(v) -> bool:
	if not v is Array or v.size() != 3:
		return false
	for x in v:
		if not _finite_number(x):
			return false
	return true


func _bookmarks(root: Node3D) -> void:
	## The review library's `bookmark` output: a Camera3D per feature under `bookmarks`, the eye at
	## the feature's `at` raised by attrs.eye_height, looking at attrs.toward's `at`, +Z of the world
	## up. A bad target or eye height is load.bad_target and no camera; no default is guessed.
	var names: Array = _manifest.get("outputs", {}).get("bookmark", [])
	var holder := Node3D.new()
	holder.name = "bookmarks"
	root.add_child(holder)
	var by_name := {}
	for f in _feature_list():
		by_name[f.get("name")] = f
	var ext: Array = _manifest.frame.extent
	var far := Vector2(float(ext[2]) - float(ext[0]), float(ext[3]) - float(ext[1])).length() + 100.0
	for name in names:
		var f: Dictionary = by_name.get(name, {})
		var attrs: Dictionary = f.get("attrs", {}) if f.get("attrs") is Dictionary else {}
		var toward = attrs.get("toward")
		var eye_h = attrs.get("eye_height")
		var bad := ""
		var why := ""
		if not _point3(f.get("at")):
			bad = "at"
			why = "it is not a point feature with a resolved `at`"
		elif not (toward is Dictionary and toward.size() == 2 and toward.get("name") is String and _point3(toward.get("at"))):
			bad = "toward"
			why = "`toward` is %s, not the resolved form {\"at\", \"name\"}" % ("missing" if toward == null else JSON.stringify(toward))
		elif not _finite_number(eye_h):
			bad = "eye_height"
			why = "`eye_height` is %s, not a finite number" % ("missing" if eye_h == null else JSON.stringify(eye_h))
		else:
			var at: Array = f.at
			var tat: Array = toward.at
			if Vector2(float(tat[0]) - float(at[0]), float(tat[1]) - float(at[1])).length() < 0.001:
				bad = "toward"
				why = "`toward` lies within 1 mm of the eye in plan"
		# `along`: a path the sheet's walker follows point by point (build.md 15.1); "" or absent
		# walks straight.
		var along = attrs.get("along", "")
		var along_pts: Array = []
		if bad == "" and along != null and along != "":
			var pf: Dictionary = by_name.get(along, {}) if along is String else {}
			# A brought bookmark's `along` is text the core does not scope (it is not in the
			# kind's `names`): read it in the bookmark's own import first (build.md 15.1).
			var dot := String(name).rfind(".")
			if along is String and dot > 0 and by_name.has("%s.%s" % [String(name).substr(0, dot), along]):
				pf = by_name["%s.%s" % [String(name).substr(0, dot), along]]
			if pf.get("shape") == "path" and pf.get("points", []).size() >= 2:
				for q in pf.points:
					along_pts.append([float(q[0]), float(q[1])])
			else:
				bad = "along"
				why = "`along` is %s, not the name of a path feature" % JSON.stringify(along)
		if bad != "":
			diags.add("load.bad_target", "feature.%s.%s" % [name, bad], String(_manifest.get("features", "features.json")),
				"bookmark %s gets no camera: %s" % [name, why],
				"check that its kind lists `toward` in `names` (authority.md 1.2) and gives the eye height a default",
				{"attribute": bad, "feature": name})
			continue
		var eye := g(float(f.at[0]), float(f.at[1]), float(f.at[2]) + float(eye_h))
		var target := _vec(toward.at)
		var cam := Camera3D.new()
		cam.name = node_name(name)
		cam.far = far
		cam.set_meta("far", far)
		cam.transform = Transform3D(Basis.looking_at(target - eye, Vector3.UP), eye)
		cam.set_meta("feature", name)
		cam.set_meta("eye", [float(f.at[0]), float(f.at[1]), float(f.at[2]) + float(eye_h)])
		cam.set_meta("toward", toward)
		cam.set_meta("along", along_pts)
		holder.add_child(cam)


func _meets(root: Node3D) -> void:
	## Each meeting: a NavigationLink3D between its two mounts, named `<a>+<b>` (mapped as names).
	var file = _manifest.get("meets")
	if not file is String:
		return
	if not FileAccess.file_exists(_dir + "/" + file):
		_bad_file(file, "missing")
		return
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(_dir + "/" + file))
	if not parsed is Dictionary or not parsed.get("meets") is Array:
		_bad_file(file, "not an object with a `meets` list")
		return
	var holder := Node3D.new()
	holder.name = "meets"
	root.add_child(holder)
	for m in parsed.meets:
		var link := NavigationLink3D.new()
		link.name = node_name("%s+%s" % [m.get("a"), m.get("b")])
		var x := float(m.get("x", 0)) / 1000.0
		var y := float(m.get("y", 0)) / 1000.0
		link.start_position = g(x, y, float(m.get("z_a", 0)))
		link.end_position = g(x, y, float(m.get("z_b", 0)))
		link.bidirectional = true
		link.set_meta("meeting", m)
		holder.add_child(link)


func _meshes(root: Node3D) -> void:
	## Each generated mesh (build.md 10): a MeshInstance3D under `meshes`, named after its `name`
	## (mapped as an instance name), from its .glb read through GLTFDocument, placed by its node's
	## translation (the mesh's south-west corner, so vertices stay small), glTF's axes being Godot's.
	## Drawn in its pack material's `colour` from `materials` (as declared, linear), or the loader's
	## default ground colour when its `material` is null or not in `materials`; a material's `file`
	## (texture) is not drawn on a mesh. With `collides`, a StaticBody3D child of the same name holding
	## a ConcavePolygonShape3D of its triangles, collided from both sides; its metadata `surface`
	## holds the mesh's name, which the walker reports. Faces are read from the glTF state's
	## ImporterMesh, which keeps them on the CPU under every display, `--headless` included.
	var list = _manifest.get("meshes")
	if not list is Array or list.is_empty():
		return
	var holder := Node3D.new()
	holder.name = "meshes"
	root.add_child(holder)
	var mats: Dictionary = _manifest.get("materials", {})
	for e in list:
		var mi := _mesh_node(e, mats)
		if mi != null:
			holder.add_child(mi)


func _mesh_node(e, mats: Dictionary) -> MeshInstance3D:
	## One entry of `meshes` as _meshes makes it; null (with load.bad_file) when it cannot be read.
	if not e is Dictionary or not e.get("file") is String or not e.get("name") is String:
		_bad_file("manifest.json", "a `meshes` entry lacks `name` or `file`")
		return null
	var file := String(e.file)
	var path := _dir + "/" + file
	if not FileAccess.file_exists(path):
		_bad_file(file, "missing")
		return null
	var doc := GLTFDocument.new()
	var state := GLTFState.new()
	if doc.append_from_file(path, state) != OK:
		_bad_file(file, "not glTF Godot reads")
		return null
	var nodes := state.get_nodes()
	var gmeshes := state.get_meshes()
	var drawn := ArrayMesh.new()
	var faces := PackedVector3Array()
	var xf := Transform3D.IDENTITY
	var found := false
	for k in nodes.size():
		var gn: GLTFNode = nodes[k]
		if gn.mesh < 0 or gn.mesh >= gmeshes.size() or found:
			continue
		found = true   # one mesh, one node (build.md 10)
		xf = gn.xform
		var p := gn.parent
		while p >= 0:
			xf = nodes[p].xform * xf
			p = nodes[p].parent
		var im: ImporterMesh = gmeshes[gn.mesh].mesh
		for s in im.get_surface_count():
			var arrays := im.get_surface_arrays(s)
			var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			var idx = arrays[Mesh.ARRAY_INDEX]
			if idx is PackedInt32Array and not idx.is_empty():
				# glTF's front faces are counter-clockwise, Godot's clockwise: GLTFDocument
				# has already turned the index order, so the faces are taken as they come.
				for t in idx:
					faces.append(verts[t])
			else:
				faces.append_array(verts)
			drawn.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	if not found:
		_bad_file(file, "no node holds a mesh")
		return null
	var mi := MeshInstance3D.new()
	mi.name = node_name(String(e.name))
	mi.mesh = drawn
	mi.transform = xf
	var m = e.get("material")
	var c: Array = mats.get(m, {}).get("colour", []) if m is String else []
	var colour := Color(c[0], c[1], c[2]) if c.size() == 3 else DEFAULT_GROUND.srgb_to_linear()
	mi.material_override = surface_material(colour)
	for key in ["feature", "surface", "material", "collides", "triangles"]:
		mi.set_meta(key, e.get(key))
	mi.set_meta("faces", faces.size() / 3)
	if e.get("collides", false) == true:
		var shape := ConcavePolygonShape3D.new()
		shape.backface_collision = true
		shape.set_faces(faces)
		var body := StaticBody3D.new()
		body.name = mi.name
		body.set_meta("surface", String(e.name))
		var cs := CollisionShape3D.new()
		cs.name = "shape"
		cs.shape = shape
		body.add_child(cs)
		mi.add_child(body)
	return mi


var _surface_materials := {}


func surface_material(colour: Color) -> ShaderMaterial:
	## A kind surface's colour as declared (linear), the way the instance shader draws an asset's
	## colour, both sides drawn (a floor is seen from the storey below).
	var key := colour.to_html()
	if not _surface_materials.has(key):
		var sh := Shader.new()
		sh.code = "shader_type spatial;\nrender_mode cull_disabled;\n#include \"res://addons/via_loader/linear.gdshaderinc\"\nuniform vec3 colour;\nvoid fragment() {\n\tALBEDO = albedo_from_linear(colour);\n\tROUGHNESS = 0.85;\n}\n"
		var mat := ShaderMaterial.new()
		mat.shader = sh
		mat.set_shader_parameter("colour", Color(colour.r, colour.g, colour.b))
		_surface_materials[key] = mat
	return _surface_materials[key]


func _keep_fields(root: Node3D, fields: Dictionary) -> Dictionary:
	## Fields the `fields` option names, as data in the root's metadata "fields": a scalar field as
	## one Image (FORMAT_RF, row 0 south); a weights field per chunk, `<cx>_<cy>` to
	## { "materials", "planes": [Image (FORMAT_R8)] }.
	var kept := {}
	var data := {}
	for sel in options.fields:
		for name in _select(fields, sel):
			kept[name] = true
	for name in kept:
		var e: Dictionary = fields[name]
		if e.get("type") == "scalar":
			var v := _scalar(name, e)
			if not v.is_empty():
				data[name] = Image.create_from_data(_nx, _ny, false, Image.FORMAT_RF, v.to_byte_array())
		elif e.get("type") == "weights":
			var per := {}
			for chunk in _manifest.get("chunks", []):
				var w: Dictionary = chunk.get("weights", {}).get(name, {})
				var at: Array = chunk.at
				var key := "%d_%d" % [int(at[0]), int(at[1])]
				var planes: Array = []
				if w.get("file") is String:
					var p: Array = _chunk_posts_of(w.file)
					var arr: Dictionary = Npy.read(_dir + "/" + w.file)
					var why: String = Npy.expect(arr, "|u1", [w.materials.size(), p[1], p[0]])
					if why != "":
						_bad_file(w.file, why)
						continue
					var plane: int = p[0] * p[1]
					for m in w.materials.size():
						planes.append(Image.create_from_data(p[0], p[1], false, Image.FORMAT_R8, arr.bytes.slice(m * plane, (m + 1) * plane)))
				per[key] = {"materials": w.get("materials", []), "planes": planes}
			data[name] = per
	root.set_meta("fields", data)
	return kept
