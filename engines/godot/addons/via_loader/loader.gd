extends RefCounted
## The Godot loader of spec/build.md 15.1: reads one build directory (format via-build-0) and makes
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

const FORMAT := "via-build-0"
## The most materials one chunk's shader draws: the layers of one Texture2DArray every renderer
## Godot ships allows (OpenGL ES 3 guarantees 256). A chunk naming more is load.too_many_materials.
const MAX_MATERIALS := 256
## Material images are resampled to one size so they share one Texture2DArray.
const ALBEDO_SIZE := 256
## A post with every weight 0 holds no material: this loader's default, the glTF reader's grey.
const DEFAULT_GROUND := Color(0.8, 0.8, 0.8)
const WATER_COLOUR := Color(0.12, 0.34, 0.48, 0.82)
const OPTION_NAMES := ["diagnostics", "features", "fields", "water"]

var diags := Diagnostics.new()
var options := {"fields": [], "water": "", "features": false, "diagnostics": ""}
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
var _water := PackedFloat32Array()
var _asset_mesh := {}
var _asset_material := {}
var _albedo := {}
var _water_material: StandardMaterial3D
var _features_cache = null


# --- options -------------------------------------------------------------------------------------

static func valid_selector(s: String) -> bool:
	var body := s
	if s.begins_with("role:"):
		body = s.trim_prefix("role:")
	elif s.begins_with("tag:"):
		body = s.trim_prefix("tag:")
	var re := RegEx.create_from_string("^[a-z][a-z0-9_]*$")
	return re.search(body) != null


static func parse_option(opts: Dictionary, name: String, value: String) -> String:
	## Sets one option from text, as the sheet tool's --option NAME=VALUE gives it. Returns "" or
	## what is wrong. A list's items are separated by `,`; `features` takes true or false.
	match name:
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
	for chunk in _manifest.get("chunks", []):
		root.add_child(_chunk(chunk, material, cols, not water.is_empty()))
	_features(root)
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
	diags.add("load.bad_file", "world", file, "%s: %s" % [file, why],
		"build again; the directory is replaced whole, so a damaged file means it was changed by hand",
		{"reason": why})


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
	arrays[Mesh.ARRAY_INDEX] = _grid_indices(cols, rows, func(_c, _r): return true)
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
	## resolution and the node scaled by it. Its rows run along +Z, which is south.
	var cols := i1 - i0 + 1
	var rows := j1 - j0 + 1
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
		var lin := Color(c[0], c[1], c[2]).srgb_to_linear()
		params.set_pixel(m, 0, Color(lin.r, lin.g, lin.b, 1.0 if img != null else 0.0))
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
		_bad_file("pack:" + String(e.file), "missing" if root != "" else "the manifest's pack_root is null and no pack directory was given")
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
	verts.resize(cols * rows)
	normals.resize(cols * rows)
	for r in rows:
		for c in cols:
			verts[r * cols + c] = g(_x0 + (i0 + c) * _res, _y0 + (j0 + r) * _res, _water[(j0 + r) * _nx + i0 + c])
			normals[r * cols + c] = Vector3.UP
	var keep := func(c, r):
		var n: int = r * cols + c
		return wet[n] == 1 or wet[n + 1] == 1 or wet[n + cols] == 1 or wet[n + cols + 1] == 1
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = _grid_indices(cols, rows, keep)
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mi := MeshInstance3D.new()
	mi.name = "water"
	mi.mesh = mesh
	mi.material_override = water_material()
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return mi


func water_material() -> StandardMaterial3D:
	if _water_material == null:
		_water_material = StandardMaterial3D.new()
		_water_material.albedo_color = WATER_COLOUR
		_water_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		_water_material.roughness = 0.25
		_water_material.metallic_specular = 0.3
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
	# A model's glTF (M2) and an engine-made asset have no mesh here; their rows are still held.
	_asset_mesh[id] = mesh
	if mesh != null:
		var mat := ShaderMaterial.new()
		mat.shader = INSTANCE_SHADER
		var c: Array = a.get("colour", [0.8, 0.8, 0.8])
		mat.set_shader_parameter("colour", Color(c[0], c[1], c[2]))
		_asset_material[id] = mat
	return mesh


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


func _bodies(node: Node3D, asset: String, first: int, count: int, cols: Dictionary) -> void:
	## Collision from the asset's params by the loader's convention (assets.md 2.2): box from
	## bounds, hull a convex shape, mesh a concave one, each on a StaticBody3D named after the
	## instance. No params.collision, no body. The scale is put into the shape, not the body.
	var a: Dictionary = _manifest.get("assets", {}).get(asset, {})
	var params = a.get("params", {})
	var kind = params.get("collision") if params is Dictionary else null
	if not kind in ["box", "hull", "mesh"] or not cols.has("names"):
		return
	var bounds: Array = a.get("bounds", [])
	if bounds.size() != 6:
		return
	var prim := "box" if a.get("stand_in", false) else String(a.get("primitive", ""))
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
				var m := Primitives.mesh(prim, bounds)
				if m == null:
					continue
				var faces := m.get_faces()
				for f in faces.size():
					faces[f] *= s
				var con := ConcavePolygonShape3D.new()
				con.set_faces(faces)
				cs.shape = con
		var body := StaticBody3D.new()
		body.name = node_name(cols.names[n])
		body.transform = _instance_transform(cols, n, false)
		body.add_child(cs)
		node.add_child(body)


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
