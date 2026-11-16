extends SceneTree
## The loader's checks against the hand-made fixture builds, no rendering needed:
##   godot --path engines/godot --rendering-method gl_compatibility --script res://tests/test_loader.gd
## Prints one line per check and exits 0 when all pass, 1 otherwise. Not --headless: MultiMesh data
## lives in the rendering server, which headless replaces with one that keeps nothing.

const Loader := preload("res://addons/via_loader/loader.gd")
const Npy := preload("res://addons/via_loader/npy.gd")
const CJson := preload("res://addons/via_loader/cjson.gd")
const Primitives := preload("res://addons/via_loader/primitives.gd")

const EXPECTED := 75
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
	_colours()
	_paths()
	_staircase()
	_edge_corner()
	_sloping_channel()
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
		and root.get_meta("id") == manifest.id and root.get_meta("format") == "via-build-1")
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
		c00.get_node_or_null("bodies/cottage/shape") != null and c00.get_node("bodies/cottage/shape").shape is BoxShape3D
		and root.get_node("c_1_0").get_node_or_null("bodies/trees#t4/shape") != null
		and root.get_node("c_1_0").get_node("bodies/trees#t4/shape").shape is ConvexPolygonShape3D
		and root.get_node("c_1_0").get_node_or_null("bodies/rocks#r0/shape") != null
		and root.get_node("c_1_0").get_node("bodies/rocks#r0/shape").shape is ConcavePolygonShape3D)
	check("before: no body without params.collision", c00.get_node_or_null("bodies/crate") == null)
	# bodies under their own `bodies` node, apart from the multimeshes named after assets: an
	# instance named like its asset keeps its name (station's `lens` was `@StaticBody3D@9`)
	var renamed := root.find_children("@*", "", true, false)
	check("before: no node renamed for a clash of names", renamed.is_empty(), str(renamed.map(func(n): return n.name)))
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
	# water's edge (15.1): a dry vertex of a drawn quad is at its wet neighbours' mean, not above
	# its ground, so the bank clips the surface; a wet vertex is at the field's value
	var wf := Npy.floats(Npy.read(dir + "/fields/water.npy"))
	var wv: PackedVector3Array = (c00.get_node("water") as MeshInstance3D).mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var shore := 0
	var shore_ok := true
	var wet_ok := true
	for j in 65:
		for i in 65:
			var k := j * 97 + i
			var z := wv[j * 65 + i].y
			if wf[k] > h[k]:
				wet_ok = wet_ok and z == wf[k]
				continue
			var s := 0.0
			var n := 0
			for dj in [-1, 0, 1]:
				for di in [-1, 0, 1]:
					var a: int = i + di
					var b: int = j + dj
					if (di != 0 or dj != 0) and a >= 0 and b >= 0 and a < 97 and b < 81 and wf[b * 97 + a] > h[b * 97 + a]:
						s += wf[b * 97 + a]
						n += 1
			if n > 0:
				shore += 1
				var deep := 0.0
				for dj in [-1, 0, 1]:
					for di in [-1, 0, 1]:
						var a: int = i + di
						var b: int = j + dj
						if (di != 0 or dj != 0) and a >= 0 and b >= 0 and a < 97 and b < 81 and wf[b * 97 + a] > h[b * 97 + a]:
							deep += wf[b * 97 + a] - h[b * 97 + a]
				var want: float = s / n if s / n <= h[k] - Loader.SHORE_GAP else h[k] - maxf(deep / n, Loader.SHORE_GAP)
				shore_ok = shore_ok and is_equal_approx(z, want) and z < h[k]
	check("before: water at wet posts is the field's value", wet_ok)
	check("before: water at the shore runs under the bank, below every dry post's ground", shore > 0 and shore_ok, "%d shore posts" % shore)
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


func _colours() -> void:
	## A pack material and an asset declare their colour in linear RGB (assets.md 2.6, 6): the
	## fixture's `rock` material and `rock_a` asset share one, and both paths must draw it as given.
	var dir := tests_dir + "/fixture_build"
	var manifest: Dictionary = _json(dir + "/manifest.json")
	var loader := Loader.new()
	var root := loader.make(dir, {})
	var declared: Array = manifest.materials.rock.colour
	var want := Color(declared[0], declared[1], declared[2])
	var names: Array = manifest.chunks[0].weights.material.materials
	var params := ((root.get_node("c_0_0/ground") as MeshInstance3D).material_override as ShaderMaterial).get_shader_parameter("params") as ImageTexture
	var px := params.get_image().get_pixel(names.find("rock"), 0)
	check("colours: a material's colour reaches the terrain shader as declared", Color(px.r, px.g, px.b).is_equal_approx(want), str(px))
	var inst := (root.get_node("c_0_0/rock_a") as MultiMeshInstance3D).material_override as ShaderMaterial
	var ic: Color = inst.get_shader_parameter("colour")
	check("colours: an asset's colour reaches the instance shader as declared, equal to the material's", ic.is_equal_approx(want)
		and manifest.assets.rock_a.colour == declared, str(ic))
	var hinted := inst.shader.get_shader_uniform_list().filter(func(u): return u.name == "colour" and u.hint == PROPERTY_HINT_COLOR_NO_ALPHA)
	check("colours: the instance shader does not convert its colour (no source_color)", hinted.is_empty())
	root.free()


func _sloping_channel() -> void:
	# A beck three posts wide at 0.5 m (massif's), falling 0.3 m a post along its line, banks
	# rising 0.04 m a post: each dry post beside the channel stands below the level of its wet
	# neighbours. Its surface must still lie strictly below its own ground, so the water's edge
	# crosses each quad edge from a wet post to it between the two posts, never on the post
	# (build.md 15.1), and the edge does not follow the posts as a row of teeth.
	var loader := Loader.new()
	loader._nx = 9
	loader._ny = 12
	loader._res = 0.5
	loader._x0 = 0.0
	loader._y0 = 0.0
	loader._height.resize(9 * 12)
	loader._water.resize(9 * 12)
	for j in 12:
		for i in 9:
			var base := 10.0 - 0.3 * j
			var ground := base + 0.04 * absi(i - 4)
			loader._height[j * 9 + i] = ground
			loader._water[j * 9 + i] = base + 0.1 if absi(i - 4) <= 1 else ground
	var mi: MeshInstance3D = loader._water_mesh(0, 0, 8, 11)
	check("sloping channel: water drawn", mi != null)
	if mi == null:
		return
	var arr := mi.mesh.surface_get_arrays(0)
	var wv: PackedVector3Array = arr[Mesh.ARRAY_VERTEX]
	var widx: PackedInt32Array = arr[Mesh.ARRAY_INDEX]
	var used := {}
	for k in widx:
		used[k] = true
	var dry := 0
	var below := true
	for k in used:
		var h: float = loader._height[k]
		if loader._water[k] > h:
			continue
		dry += 1
		below = below and wv[k].y < h - 1e-4
	check("sloping channel: every dry post of a drawn quad lies below its ground", dry > 0 and below, "%d dry posts" % dry)
	# Along each drawn edge from a wet post to a dry one, the depth crosses zero strictly between
	# the posts, and nowhere within a tenth of the way of the dry post.
	var between := true
	for t in range(0, widx.size(), 3):
		for e in [[0, 1], [1, 2], [2, 0]]:
			var p: int = widx[t + e[0]]
			var q: int = widx[t + e[1]]
			var dp: float = wv[p].y - loader._height[p]
			var dq: float = wv[q].y - loader._height[q]
			if (dp > 0.0) == (dq > 0.0):
				continue
			var f := dp / (dp - dq)
			var to_dry := f if dq <= 0.0 else 1.0 - f
			between = between and to_dry > 0.0 and to_dry < 0.9
	check("sloping channel: the edge falls between posts, not on them", between)
	mi.free()


func _staircase() -> void:
	# A found course's staircase: 0.2 m steps alternating east and north-east, then a 45-degree
	# turn within half a metre, under a strip 5 m wide (massif's road). No triangle of the strip may
	# turn over: every one keeps the winding of the first, seen from above.
	var dir := tests_dir + "/fixture_build"
	var loader := Loader.new()
	var root := loader.make(dir, {})
	if root == null:
		check("staircase: loads", false, loader.option_error)
		return
	var x0: float = loader._x0 + 20.0
	var y0: float = loader._y0 + 20.0
	var pts: Array = []
	var p := Vector2(x0, y0)
	for k in 30:
		pts.append([p.x, p.y])
		p += Vector2(0.2, 0.0) if k % 2 == 0 else Vector2(0.15, 0.15)
	for k in 20:
		pts.append([p.x, p.y])
		p += Vector2(0.0, 0.25)
	pts.append([p.x, p.y])
	var mesh: ArrayMesh = loader._strip(pts, 5.0)
	var arr := mesh.surface_get_arrays(0)
	var v: PackedVector3Array = arr[Mesh.ARRAY_VERTEX]
	var idx: PackedInt32Array = arr[Mesh.ARRAY_INDEX]
	var across := Loader.across_count(5.0, loader._res)
	# The strip's rows: every half post spacing of the line's length, both ends included; its
	# triangles come first in the index list, the corner discs after.
	var length := 0.0
	for k in range(1, pts.size()):
		length += Vector2(pts[k][0] - pts[k - 1][0], pts[k][1] - pts[k - 1][1]).length()
	var rows := maxi(1, ceili(length / (loader._res * 0.5))) + 1
	var tris := (rows - 1) * (across - 1) * 2
	var first := 0.0
	var flipped := 0
	for t in tris:
		var a3 := v[idx[3 * t]]
		var b3 := v[idx[3 * t + 1]]
		var c3 := v[idx[3 * t + 2]]
		# in plan, (x, y) = (X, -Z)
		var cr := (b3.x - a3.x) * (a3.z - c3.z) - (a3.z - b3.z) * (c3.x - a3.x)
		if first == 0.0:
			first = signf(cr)
		elif signf(cr) != first and absf(cr) > 1e-9:
			flipped += 1
	check("staircase: the strip has its rows", idx.size() >= 3 * tris and tris > 0, str([tris, idx.size() / 3]))
	check("staircase: no triangle of the strip turns over", flipped == 0, "%d of %d" % [flipped, tris])
	root.free()


func _edge_corner() -> void:
	# A path that starts on the world's south edge and turns sharply one metre in, within half its
	# width of the edge (massif's valley_road at (314, 0)): no vertex of its strip or of the corner
	# disc lies outside the extent, and the strip still covers the corner.
	var dir := tests_dir + "/fixture_build"
	var loader := Loader.new()
	var root := loader.make(dir, {})
	if root == null:
		check("edge corner: loads", false, loader.option_error)
		return
	var x0: float = loader._x0
	var y0: float = loader._y0
	var x1: float = x0 + (loader._nx - 1) * loader._res
	var y1: float = y0 + (loader._ny - 1) * loader._res
	var cx := x0 + 0.5 * (x1 - x0)
	var pts: Array = [[cx, y0], [cx, y0 + 1.0], [cx + 6.0, y0 + 4.5]]
	var mesh: ArrayMesh = loader._strip(pts, 5.0)
	var v: PackedVector3Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var outside := 0
	var worst := 0.0
	for q in v:
		var px := q.x
		var py := -q.z
		var d := maxf(maxf(x0 - px, px - x1), maxf(y0 - py, py - y1))
		if d > 1e-4:
			outside += 1
			worst = maxf(worst, d)
	check("edge corner: no path vertex outside the extent", outside == 0, "%d, by %.3f m" % [outside, worst])
	var on_edge := false
	for q in v:
		if absf(-q.z - y0) < 1e-4 and absf(q.x - cx) < 2.6:
			on_edge = true
	check("edge corner: the strip reaches the edge it starts on", on_edge)
	root.free()


func _paths() -> void:
	var dir := tests_dir + "/fixture_build"
	var feats: Array = _json(dir + "/features.json").features
	var loader := Loader.new()
	var root := loader.make(dir, {"paths": ["kind:lane"]})
	check("paths: loads", root != null, loader.option_error)
	if root == null:
		return
	var held := root.get_node("paths").get_children().map(func(n): return String(n.name))
	check("paths: one strip per lane, the beck not drawn", held == ["lane_east", "lane_west"], str(held))
	var west: Dictionary = feats.filter(func(x): return x.name == "lane_west")[0]
	var west_mi := root.get_node("paths/lane_west") as MeshInstance3D
	var v: PackedVector3Array = west_mi.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var n: int = west_mi.get_meta("across")
	var res: float = loader._res
	check("paths: vertices across no further apart than the post spacing",
		n >= 3 and n % 2 == 1 and float(west.attrs.width) / (n - 1) <= res + 1e-6, str([n, res]))
	var p0: Array = west.points[0]
	var mid := v[(n - 1) / 2]
	check("paths: the strip runs from the lane's first point, at least the lift over the ground",
		is_equal_approx(mid.x, float(p0[0])) and is_equal_approx(-mid.z, float(p0[1]))
		and mid.y >= loader.ground_at(float(p0[0]), float(p0[1])) + Loader.PATH_LIFT - 1e-5, str(mid))
	# No ground post stands above the ribbon: at every post inside a ribbon triangle, and at its
	# centre, the triangle is above the ground mesh.
	var widx: PackedInt32Array = west_mi.mesh.surface_get_arrays(0)[Mesh.ARRAY_INDEX]
	var tris_w := (v.size() / n - 1) * (n - 1) * 2
	var through := 0
	for t in tris_w:
		var a3 := v[widx[3 * t]]
		var b3 := v[widx[3 * t + 1]]
		var c3 := v[widx[3 * t + 2]]
		var pa := Vector2(a3.x, -a3.z)
		var pb := Vector2(b3.x, -b3.z)
		var pc := Vector2(c3.x, -c3.z)
		var den := (pb.y - pc.y) * (pa.x - pc.x) + (pc.x - pb.x) * (pa.y - pc.y)
		if absf(den) < 1e-12:
			continue
		var qs: Array[Vector2] = [(pa + pb + pc) / 3.0]
		for j in range(floori((minf(pa.y, minf(pb.y, pc.y)) - loader._y0) / res), ceili((maxf(pa.y, maxf(pb.y, pc.y)) - loader._y0) / res) + 1):
			for i in range(floori((minf(pa.x, minf(pb.x, pc.x)) - loader._x0) / res), ceili((maxf(pa.x, maxf(pb.x, pc.x)) - loader._x0) / res) + 1):
				qs.append(Vector2(loader._x0 + i * res, loader._y0 + j * res))
		for q in qs:
			var w0 := ((pb.y - pc.y) * (q.x - pc.x) + (pc.x - pb.x) * (q.y - pc.y)) / den
			var w1 := ((pc.y - pa.y) * (q.x - pc.x) + (pa.x - pc.x) * (q.y - pc.y)) / den
			if w0 < 0.0 or w1 < 0.0 or w0 + w1 > 1.0:
				continue
			if loader.ground_at(q.x, q.y) > w0 * a3.y + w1 * b3.y + (1.0 - w0 - w1) * c3.y + 1e-4:
				through += 1
	check("paths: no ground post stands above the ribbon", through == 0, "%d" % through)
	# Measured a few rows in: the lane starts on the world's edge, where its first rows end along it.
	var r4 := 4 * n
	check("paths: as wide as the lane's width attribute", is_equal_approx(Vector2(v[r4].x - v[r4 + n - 1].x, v[r4].z - v[r4 + n - 1].z).length(), float(west.attrs.width)))
	var pe: Array = west.points[-1]
	var last := v[v.size() - n + (n - 1) / 2]
	check("paths: the strip ends at the lane's last point", is_equal_approx(last.x, float(pe[0])) and is_equal_approx(-last.z, float(pe[1])), str(last))
	# The lanes meet at the ford: one join, drawn once, by the first lane by name, centred there.
	var east_mi := root.get_node("paths/lane_east") as MeshInstance3D
	check("paths: lanes sharing an end are joined there once", east_mi.get_meta("joins") == 1 and west_mi.get_meta("joins") == 0,
		str([east_mi.get_meta("joins"), west_mi.get_meta("joins")]))
	var ev: PackedVector3Array = east_mi.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var east: Dictionary = feats.filter(func(x): return x.name == "lane_east")[0]
	var ep: Array = east.points[0]
	var centre_at := -1   # the last vertex there: the strip's first row has one too
	for k in ev.size():
		if is_equal_approx(ev[k].x, float(ep[0])) and is_equal_approx(-ev[k].z, float(ep[1])):
			centre_at = k
	check("paths: the join is a disc at the shared point, after the strip", centre_at >= 0 and centre_at % n == 0 and centre_at > n,
		str([centre_at, n]))
	if centre_at >= 0:
		var rim := ev[ev.size() - 1]   # on the outer ring
		check("paths: the join is as wide as the lane", is_equal_approx(Vector2(rim.x - ev[centre_at].x, rim.z - ev[centre_at].z).length(), float(east.attrs.width) * 0.5),
			str(rim))
	# Draped on the ground as its mesh draws it: each quad split from south-west to north-east.
	var x0: float = loader._x0
	var y0: float = loader._y0
	var h00: float = loader._h(3, 4)
	var h10: float = loader._h(4, 4)
	var h11: float = loader._h(4, 5)
	var h01: float = loader._h(3, 5)
	var on_se := loader.ground_at(x0 + 3.75 * res, y0 + 4.25 * res)
	var on_nw := loader.ground_at(x0 + 3.25 * res, y0 + 4.75 * res)
	check("paths: ground_at follows the ground mesh's triangles",
		is_equal_approx(on_se, h00 + 0.75 * (h10 - h00) + 0.25 * (h11 - h10))
		and is_equal_approx(on_nw, h00 + 0.75 * (h01 - h00) + 0.25 * (h11 - h01)), str([on_se, on_nw]))
	root.free()
	var none := Loader.new()
	check("paths: a selector matching no path feature is an option error",
		none.make(dir, {"paths": ["kind:road"]}) == null and none.option_error.begins_with("paths:"), none.option_error)
	var opts := {}
	check("option: paths is a list of names and kind:<kind>", Loader.parse_option(opts, "paths", "kind:lane,beck") == ""
		and opts.paths == ["kind:lane", "beck"])
	check("option: paths refuses a field selector", Loader.parse_option(opts, "paths", "role:surface") != "")


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
