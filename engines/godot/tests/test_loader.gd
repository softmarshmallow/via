extends SceneTree
## The loader's checks against the hand-made fixture builds, no rendering needed:
##   godot --path engines/godot --rendering-method gl_compatibility --script res://tests/test_loader.gd
## Prints one line per check and exits 0 when all pass, 1 otherwise. Not --headless: MultiMesh data
## lives in the rendering server, which headless replaces with one that keeps nothing.

const Loader := preload("res://addons/via_loader/loader.gd")
const Npy := preload("res://addons/via_loader/npy.gd")
const CJson := preload("res://addons/via_loader/cjson.gd")
const Primitives := preload("res://addons/via_loader/primitives.gd")

const EXPECTED := 55
var failed := 0
var ran := 0
var tests_dir := ""


func _initialize() -> void:
	tests_dir = ProjectSettings.globalize_path("res://tests")
	_primitives()
	_cjson()
	_cjson_exact()
	_before()
	_after()
	_no_manifest()
	_bad_format()
	_options()
	# a script error skips checks silently; count them so a skip fails
	check("every check ran", ran >= EXPECTED, "%d of %d" % [ran, EXPECTED])
	print("%s: %d check(s) failed" % ["FAIL" if failed else "PASS", failed])
	quit(1 if failed else 0)


func check(what: String, ok: bool, detail := "") -> void:
	ran += 1
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


func _json(path: String):
	return JSON.parse_string(FileAccess.get_file_as_string(path))


func _primitives() -> void:
	var want := {"box": 12, "prism": 8, "cylinder": 60, "cone": 30, "sphere": 224}
	for p in want:
		var tris := Primitives.triangles(p, [-1, -1, 0, 1, 1, 2])
		check("primitive %s has %d triangles" % [p, want[p]], tris.size() == want[p], str(tris.size()))
		# every face turned outward: the mesh encloses positive volume
		var vol := 0.0
		for t in tris:
			vol += t[0].dot(t[1].cross(t[2])) / 6.0
		check("primitive %s faces outward" % p, vol > 0, str(vol))
	var m := Primitives.mesh("box", [-1, -1, 0, 1, 1, 2])
	var aabb := m.get_aabb()
	check("box mesh fills bounds in Godot axes (x, z, -y)", aabb.position.is_equal_approx(Vector3(-1, 0, -1)) and aabb.size.is_equal_approx(Vector3(2, 2, 2)), str(aabb))


func _cjson() -> void:
	check("canonical JSON sorts keys and writes shortest numbers",
		CJson.encode({"b": 1.5, "a": [0.1, 2.0, -0.0, null, true], "c": 153.63}) == '{"a":[0.1,2,0,null,true],"b":1.5,"c":153.63}',
		CJson.encode({"b": 1.5, "a": [0.1, 2.0, -0.0, null, true], "c": 153.63}))


func _cjson_exact() -> void:
	# a single-precision value widened exactly; the shortest form ends on a tie, read back to even
	var f := PackedFloat32Array([-108.8356018066406])
	check("canonical JSON: shortest digits, exactly, ties to even", CJson.number(f[0]) == "-108.83560180664062", CJson.number(f[0]))
	check("canonical JSON: an integral double is an integer", CJson.number(1e15) == "1000000000000000" and CJson.number(-0.0) == "0")


func _before() -> void:
	var dir := tests_dir + "/fixture_build"
	var manifest: Dictionary = _json(dir + "/manifest.json")
	var loader := Loader.new()
	var root := loader.make(dir, {"water": "tag:water_level", "features": true, "fields": ["clearing"]})
	check("before: loads", root != null, loader.option_error)
	if root == null:
		return
	check("before: root named after world, holding id and format", root.name == "fixture"
		and root.get_meta("id") == manifest.id and root.get_meta("format") == "via-build-0")
	check("before: no error", loader.diags.errors() == 0, str(loader.diags.records))
	# one chunk node per chunk; one MultiMeshInstance3D per chunk entry's asset holding its count
	var counts_ok := true
	var total := 0
	for chunk in manifest.chunks:
		var node := root.get_node_or_null("c_%d_%d" % [int(chunk.at[0]), int(chunk.at[1])])
		if node == null or node.get_node_or_null("ground") == null or node.get_node_or_null("ground_body/shape") == null:
			counts_ok = false
			continue
		for asset in chunk.instances:
			var mmi := node.get_node_or_null(NodePath(asset)) as MultiMeshInstance3D
			if mmi == null or mmi.multimesh.instance_count != int(chunk.instances[asset][1]) or mmi.multimesh.mesh == null:
				counts_ok = false
			else:
				total += mmi.multimesh.instance_count
	check("before: chunk nodes with ground, collision and one MultiMesh per asset holding its count", counts_ok)
	check("before: every instance row is held", total == int(manifest.instances.count), "%d of %d" % [total, manifest.instances.count])
	# a set item's name keeps its row findable, `.` mapped to `-` on its body
	var c00 := root.get_node("c_0_0")
	var fence := c00.get_node("fence_post") as MultiMeshInstance3D
	check("before: names in metadata keep rows findable", "fence.p2" in fence.get_meta("names"))
	check("before: collision bodies from params (box, hull, mesh)",
		c00.get_node_or_null("cottage/shape") != null and c00.get_node("cottage/shape").shape is BoxShape3D
		and root.get_node("c_1_0").get_node_or_null("trees#t4/shape") != null
		and root.get_node("c_1_0").get_node("trees#t4/shape").shape is ConvexPolygonShape3D
		and root.get_node("c_1_0").get_node_or_null("rocks#r0/shape") != null
		and root.get_node("c_1_0").get_node("rocks#r0/shape").shape is ConcavePolygonShape3D)
	check("before: no body without params.collision", c00.get_node_or_null("crate") == null)
	# instance transform: pivot and facing (face 90000 turns +y front to east)
	var cottage_row := Array(fence.get_meta("names")).size()
	var house := c00.get_node("house_low") as MultiMeshInstance3D
	var t := house.multimesh.get_instance_transform(0)
	var front := t.basis * Primitives.g(Vector3(0, 1, 0))
	check("before: face 90 turns the asset's front to east", front.is_equal_approx(Vector3(1, 0, 0)), str(front))
	check("before: cottage pivot at (30, 52) m", t.origin.x == 30.0 and t.origin.z == -52.0, str(t.origin))
	var tint: float = c00.get_node("box_small").multimesh.get_instance_custom_data(0).r
	check("before: tint as instance custom data", absf(tint - 0.9) < 1e-3, str(tint))
	# ground material: shader count equals the chunk's materials; the weights as a Texture2DArray
	var mat := c00.get_node("ground").material_override as ShaderMaterial
	var arr := mat.get_shader_parameter("weights") as Texture2DArray
	check("before: chunk 0_0 draws 4 materials from one R8 Texture2DArray", mat.get_shader_parameter("count") == 4
		and arr != null and arr.get_layers() == 4 and arr.get_format() == Image.FORMAT_R8)
	var mat01 := root.get_node("c_0_1/ground").material_override as ShaderMaterial
	check("before: chunk 0_1 draws its own 3", mat01.get_shader_parameter("count") == 3)
	check("before: a material's file is drawn from the pack", mat.get_shader_parameter("albedo") is Texture2DArray)
	# ground mesh heights equal the field at a post
	var h := Npy.floats(Npy.read(dir + "/fields/height.npy"))
	var verts: PackedVector3Array = (c00.get_node("ground") as MeshInstance3D).mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	check("before: ground vertex (i 10, j 20) at the field's height", verts[20 * 65 + 10] == Vector3(10, h[20 * 97 + 10], -20), str(verts[20 * 65 + 10]))
	var hm := (root.get_node("c_1_1/ground_body/shape") as CollisionShape3D)
	var hshape := hm.shape as HeightMapShape3D
	check("before: last chunk's height map is narrower (33 by 17) and centred", hshape.map_width == 33 and hshape.map_depth == 17
		and hm.position.is_equal_approx(Vector3(80, 0, -72)), "%d x %d at %s" % [hshape.map_width, hshape.map_depth, hm.position])
	check("before: height map row 0 is the chunk's north row", is_equal_approx(hshape.map_data[0], h[80 * 97 + 64]))
	# water: drawn where the beck is, not in the dry chunk
	check("before: water mesh in chunks the beck crosses", root.get_node_or_null("c_0_0/water") != null and root.get_node_or_null("c_1_0/water") != null)
	# features, footprints, bookmarks, meetings
	check("before: features as Path3D, Marker3D and polygon metadata", root.get_node("features/beck") is Path3D
		and root.get_node("features/ford") is Marker3D and root.get_node("features/meadow").get_meta("polygon").size() == 1
		and root.get_node("features/beck").get_meta("attrs").get("width") == 3)
	check("before: a footprint per grounded instance", root.get_node_or_null("features/footprints/cottage") != null)
	var cams := root.get_node("bookmarks").get_children()
	check("before: four bookmark cameras", cams.size() == 4 and cams.all(func(c): return c is Camera3D), str(cams.size()))
	var cam := root.get_node("bookmarks/at_ford") as Camera3D
	var feats: Array = _json(dir + "/features.json").features
	var f: Dictionary = feats.filter(func(x): return x.name == "at_ford")[0]
	var eye := Vector3(f.at[0], f.at[2] + 1.7, -f.at[1])
	var target := Vector3(f.attrs.toward.at[0], f.attrs.toward.at[2], -f.attrs.toward.at[1])
	check("before: camera at the eye, raised by eye_height", cam.position.is_equal_approx(eye), str(cam.position))
	check("before: camera looks at toward's at", (-cam.transform.basis.z).is_equal_approx((target - eye).normalized()))
	check("before: camera defaults kept, far is the diagonal plus 100 m", is_equal_approx(cam.fov, 75.0) and is_equal_approx(cam.near, 0.05)
		and is_equal_approx(cam.far, Vector2(96, 80).length() + 100.0))
	check("before: meetings as NavigationLink3D", root.get_node("meets").get_child_count() == 2
		and root.get_node_or_null("meets/ford+lane_west-end") is NavigationLink3D)
	# fields kept and not drawn
	var kept: Dictionary = root.get_meta("fields")
	check("before: a kept field is an Image (FORMAT_RF)", kept.has("clearing") and kept.clearing is Image
		and kept.clearing.get_format() == Image.FORMAT_RF and kept.clearing.get_width() == 97)
	var nd := loader.diags.records.filter(func(r): return r.code == "load.fields_not_drawn")
	check("before: nothing listed as not drawn when every field is drawn or kept", nd.is_empty(), str(nd))
	root.free()
	var plain := Loader.new()
	var r2 := plain.make(dir, {})
	nd = plain.diags.records.filter(func(r): return r.code == "load.fields_not_drawn")
	check("before, no options: load.fields_not_drawn lists clearing and water once", nd.size() == 1
		and nd[0].data.fields == ["clearing", "water"] and nd[0].severity == "info", str(nd))
	check("before, no options: no water, no features", r2.get_node_or_null("c_0_0/water") == null and r2.get_node_or_null("features") == null)
	r2.free()


func _after() -> void:
	var dir := tests_dir + "/fixture_build+downstream"
	var loader := Loader.new()
	var out := ProjectSettings.globalize_path("user://after_diagnostics.json")
	var root := loader.load_build(dir, {"water": "tag:water_level", "diagnostics": out})
	check("after: loads", root != null)
	var bad := loader.diags.records.filter(func(r): return r.code == "load.bad_target")
	check("after: a plain-text toward is load.bad_target and no camera", bad.size() == 1
		and bad[0].subject == "feature.broken.toward" and root.get_node_or_null("bookmarks/broken") == null, str(bad))
	check("after: the other five cameras are made", root.get_node("bookmarks").get_child_count() == 5)
	var doc = _json(out)
	check("after: diagnostics written as diagnostics.md 7's document", doc is Dictionary and doc.format == "via-diagnostics-1"
		and doc.status == 0 and doc.counts.warning == 1 and doc.diagnostics[0].code == "load.bad_target")
	check("after: the document is canonical", FileAccess.get_file_as_string(out) == CJson.text(doc))
	root.free()


func _no_manifest() -> void:
	var dir := ProjectSettings.globalize_path("user://no_manifest")
	DirAccess.make_dir_recursive_absolute(dir)
	var f := FileAccess.open(dir + "/diagnostics.json", FileAccess.WRITE)
	f.store_string("{}\n")
	f.close()
	var loader := Loader.new()
	var root := loader.make(dir)
	check("no manifest: load.format naming diagnostics.json", root == null and loader.diags.records.size() == 1
		and loader.diags.records[0].code == "load.format" and loader.diags.records[0].file == "diagnostics.json")


func _bad_format() -> void:
	var dir := ProjectSettings.globalize_path("user://bad_format")
	DirAccess.make_dir_recursive_absolute(dir)
	var f := FileAccess.open(dir + "/manifest.json", FileAccess.WRITE)
	f.store_string('{"format":"via-build-9"}\n')
	f.close()
	var loader := Loader.new()
	check("unknown format: load.format", loader.make(dir) == null and loader.diags.records[0].code == "load.format")


func _options() -> void:
	var opts := {}
	check("option: unknown name refused", Loader.parse_option(opts, "colour", "x") != "")
	check("option: bad selector refused", Loader.parse_option(opts, "water", "Tag:Water") != "")
	check("option: features takes true or false", Loader.parse_option(opts, "features", "yes") != ""
		and Loader.parse_option(opts, "features", "true") == "" and opts.features == true)
	check("option: fields is a list", Loader.parse_option(opts, "fields", "clearing,tag:water_level") == ""
		and opts.fields == ["clearing", "tag:water_level"])
	var loader := Loader.new()
	check("option: a water selector matching no field is an option error",
		loader.make(tests_dir + "/fixture_build", {"water": "tag:lava"}) == null and loader.option_error != "")
