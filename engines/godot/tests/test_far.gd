extends SceneTree
## The streamer's far layer (spec/build.md 15.1, "Streaming": far ground, far water, skirts, far
## instances, far meshes), headless:
##   godot --headless --path engines/godot --script res://tests/test_far.gd -- \
##         --build DIR --water SELECTOR --meshes DIR
## DIR is fixture_build (four chunks, a water field, primitives); --meshes a build with generated
## meshes (the floor fixture). Checks:
## - the far ground's posts lie at most Loader.FAR_STEP apart in plan;
## - each far edge carries a skirt reaching below every fine post of that edge (no slit);
## - with the water option, the far water covers every wet post sampled on the far grid, and
##   without it there is none;
## - a far instance is its asset's outline by bands: a box is one box of its bounds, a cone's bands
##   narrow upward inside its bounds, a sphere's rings widen to its middle and narrow to its top,
##   each an octagon (its surface's diagonal extents cut its box's corners) where a box keeps its
##   four, each band a frustum between its rings;
## - every generated mesh has a copy with no body, shown exactly while the mesh is not loaded.
## Prints one line per check; exits 0 when all pass.

const Loader := preload("res://addons/via_loader/loader.gd")
const Stream := preload("res://addons/via_loader/stream.gd")
var failed := 0


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var dir := ""
	var water := ""
	var meshes := ""
	var k := 0
	while k + 1 < args.size():
		match args[k]:
			"--build": dir = args[k + 1]
			"--water": water = args[k + 1]
			"--meshes": meshes = args[k + 1]
		k += 2
	var opts := {}
	Loader.parse_option(opts, "water", water)
	var loader := Loader.new()
	var root := loader.prepare(dir, opts)
	var far := loader.make_far_ground()
	var arrays := far.mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var idx: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	# posts at most FAR_STEP apart: every triangle edge of the top surface (normals up) is short
	var longest := 0.0
	var skirt_tris := 0
	for t in idx.size() / 3:
		var a := verts[idx[3 * t]]
		var b := verts[idx[3 * t + 1]]
		var c := verts[idx[3 * t + 2]]
		if absf((b - a).cross(c - a).normalized().y) < 0.5:
			skirt_tris += 1
			continue
		for e in [[a, b], [b, c], [c, a]]:
			var d: Vector3 = e[1] - e[0]
			longest = maxf(longest, maxf(absf(d.x), absf(d.z)))
	check("far ground: posts at most %.0f m apart in plan" % Loader.FAR_STEP, longest <= Loader.FAR_STEP + 1e-3, "%.2f m" % longest)
	# skirts: for each fine post on a chunk edge, a skirt triangle spans its plan point from at or
	# above the post down to below it
	var uncovered := 0
	var posts := 0
	for chunk in loader.chunk_list():
		var p: Array = chunk.posts
		for i in range(int(p[0]), int(p[2]) + 1):
			for j in [int(p[1]), int(p[3])]:
				posts += 1
				if not _skirted(verts, idx, loader, i, j):
					uncovered += 1
		for j in range(int(p[1]), int(p[3]) + 1):
			for i in [int(p[0]), int(p[2])]:
				posts += 1
				if not _skirted(verts, idx, loader, i, j):
					uncovered += 1
	check("skirts: every fine post on a chunk edge has its far edge's skirt reaching below it", uncovered == 0 and skirt_tris > 0,
		"%d of %d posts uncovered; %d skirt triangles" % [uncovered, posts, skirt_tris])
	# far water
	var fw: MeshInstance3D = far.get_node_or_null("far_water")
	check("far water: made with the water option", fw != null)
	if fw != null:
		var w: PackedVector3Array = fw.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
		var wet := 0
		var missed := 0
		for i in range(0, loader._nx, 4):
			for j in range(0, loader._ny, 4):
				var x: float = loader._x0 + i * loader._res
				var y: float = loader._y0 + j * loader._res
				if loader.depth_at(x, y) <= 0.0:
					continue
				wet += 1
				if not _under(w, Loader.g(x, y, 0)):
					missed += 1
		check("far water: covers every wet post on a 4-post grid", wet > 0 and missed == 0, "%d wet, %d not covered" % [wet, missed])
	var dry := Loader.new()
	var droot := dry.prepare(dir, {})
	var df := dry.make_far_ground()
	check("far water: none without the water option", df.get_node_or_null("far_water") == null)
	df.free()
	droot.free()
	far.free()
	# far parts
	var box = loader._far_parts("house_low")
	check("far instances: a box is one box of its bounds", box is Array and box.size() == 1 and box[0][0] == [-4, -3, 0, 4, 3, 4] or box is Array and box.size() == 1
		and Vector3(box[0][0][0], box[0][0][1], box[0][0][2]).is_equal_approx(Vector3(-4, -3, 0)) and Vector3(box[0][0][3], box[0][0][4], box[0][0][5]).is_equal_approx(Vector3(4, 3, 4)), str(box))
	var cone = loader._far_parts("fern_low")
	var narrows: bool = cone is Array and cone.size() == Loader.FAR_BANDS
	if narrows:
		for n in range(1, cone.size()):
			narrows = narrows and float(cone[n][0][3]) - float(cone[n][0][0]) < float(cone[n - 1][0][3]) - float(cone[n - 1][0][0])
		for part in cone:
			narrows = narrows and float(part[0][0]) >= -0.4 - 1e-3 and float(part[0][3]) <= 0.4 + 1e-3
	check("far instances: a cone's bands narrow upward, inside its bounds", narrows, str(cone))
	var ball = loader._far_parts("tree_oak")
	# M7: rings at the band ends; a sphere's widen from its foot to its middle and narrow to its
	# top, its middle rings octagons inside its box; a box's ring is its box
	var rings: Array = []
	if ball is Array and ball.size() == 3:
		rings = [ball[0][3], ball[1][3], ball[2][3], ball[2][4]]
	var w := rings.map(func(r): return float(r[2]) - float(r[0]))
	check("far instances: a sphere's rings widen to its middle and narrow to its top", rings.size() == 4
		and w[1] > w[0] and w[2] > w[3], "%s" % [w])
	var mid_ring := Loader.far_outline(rings[1]) if rings.size() == 4 else PackedVector2Array()
	var inside := mid_ring.size() == 8
	for q in mid_ring:
		inside = inside and q.x >= float(rings[1][0]) - 1e-3 and q.x <= float(rings[1][2]) + 1e-3 \
			and q.y >= float(rings[1][1]) - 1e-3 and q.y <= float(rings[1][3]) + 1e-3
	var box_ring := Loader.far_outline(box[0][3]) if box is Array and box[0].size() > 3 else PackedVector2Array()
	check("far instances: a box's ring is its box, a sphere's middle ring an octagon inside its box",
		box_ring.size() == 4 and inside, "%s; %s" % [box_ring, mid_ring])
	# the far mesh of a sphere: a frustum per band, no face left open between bands
	var vs := PackedVector3Array()
	var ns := PackedVector3Array()
	var cs := PackedColorArray()
	var us := PackedVector2Array()
	for part in (ball if ball is Array else []):
		Loader._far_frustum(vs, ns, cs, us, Transform3D.IDENTITY, part, Color(1, 1, 1), Vector2.ZERO)
	var top_y := -INF
	for v in vs:
		top_y = maxf(top_y, v.y)
	check("far instances: a sphere's bands drawn as frusta up to its top", vs.size() > 0 and vs.size() % 3 == 0
		and is_equal_approx(top_y, float(ball[2][0][5])), "%d vertices, top %.3f" % [vs.size(), top_y])
	root.free()
	# far meshes
	if meshes != "":
		var ml := Loader.new()
		var mroot := ml.prepare(meshes, {})
		get_root().add_child(mroot)
		var stream = Stream.new(ml, mroot, 5.0)
		var n_meshes: int = ml.mesh_list().size()
		var holder: Node = mroot.get_node_or_null("far_meshes")
		var bodies := 0
		if holder != null:
			for m in holder.get_children():
				bodies += m.get_child_count()
		check("far meshes: one copy with no body for each generated mesh", n_meshes > 0 and stream.far_mesh_count() == n_meshes and bodies == 0,
			"%d of %d, %d bodies" % [stream.far_mesh_count(), n_meshes, bodies])
		stream.fill(Vector2.ZERO, INF)
		var shown := 0
		for m in holder.get_children():
			shown += 1 if m.visible else 0
		check("far meshes: hidden while their meshes are loaded", shown == 0, "%d shown" % shown)
		stream.update(Vector2(1e9, 1e9))
		shown = 0
		for m in holder.get_children():
			shown += 1 if m.visible else 0
		check("far meshes: shown again once unloaded", shown == n_meshes, "%d of %d shown" % [shown, n_meshes])
		stream.end()
		mroot.free()
	print("%s: %d check(s) failed" % ["PASS" if failed == 0 else "FAIL", failed])
	quit(0 if failed == 0 else 1)


static func _under(tris: PackedVector3Array, p: Vector3) -> bool:
	## Whether some triangle of the soup covers plan point p.
	var q := Vector2(p.x, p.z)
	for t in tris.size() / 3:
		if Geometry2D.point_is_inside_triangle(q, Vector2(tris[3 * t].x, tris[3 * t].z), Vector2(tris[3 * t + 1].x, tris[3 * t + 1].z),
				Vector2(tris[3 * t + 2].x, tris[3 * t + 2].z)):
			return true
		# on an edge: within a millimetre of the triangle
		var a := Vector2(tris[3 * t].x, tris[3 * t].z)
		var b := Vector2(tris[3 * t + 1].x, tris[3 * t + 1].z)
		var c := Vector2(tris[3 * t + 2].x, tris[3 * t + 2].z)
		for e in [[a, b], [b, c], [c, a]]:
			if Geometry2D.get_closest_point_to_segment(q, e[0], e[1]).distance_to(q) < 1e-3:
				return true
	return false


static func _skirted(verts: PackedVector3Array, idx: PackedInt32Array, loader, i: int, j: int) -> bool:
	## A vertical (skirt) triangle whose plan segment passes the post and reaches below it (or a
	## far vertex standing on the post itself): where the post stands below its far edge, the slit
	## between them is closed; where above, the fine ground hides the far edge from the loaded side.
	var p: Vector3 = Loader.g(loader._x0 + i * loader._res, loader._y0 + j * loader._res, loader._h(i, j))
	var q := Vector2(p.x, p.z)
	for t in idx.size() / 3:
		var a := verts[idx[3 * t]]
		var b := verts[idx[3 * t + 1]]
		var c := verts[idx[3 * t + 2]]
		if absf((b - a).cross(c - a).normalized().y) >= 0.5:
			if Vector2(a.x, a.z).distance_to(q) < 1e-3 and absf(a.y - p.y) < 1e-3:
				return true
			continue
		var lo := minf(a.y, minf(b.y, c.y))
		var hi := maxf(a.y, maxf(b.y, c.y))
		var pts := [Vector2(a.x, a.z), Vector2(b.x, b.z), Vector2(c.x, c.z)]
		for e in [[0, 1], [1, 2], [2, 0]]:
			if Geometry2D.get_closest_point_to_segment(q, pts[e[0]], pts[e[1]]).distance_to(q) < 1e-3 and lo < p.y - 1e-4:
				return true
	return false
