extends RefCounted
## The five primitive shapes of spec/assets.md 6, made as meshes in Godot's axes. Triangle counts
## are fixed by that chapter and bind whoever draws them: box 12, prism 8, cylinder 60, cone 30,
## sphere 224. The shape fills `bounds` (asset frame: x east, y north, z up; the pivot at the
## origin), and a 16-sided ring has its vertex k at k * 22.5 degrees counter-clockwise from +x.
## Every shape is convex, so each triangle is turned to face away from the centre of `bounds`.

const SIDES := 16
const SPHERE_ROWS := 8


static func g(p: Vector3) -> Vector3:
	## Asset or world frame to Godot: (x, y, z) -> (x, z, -y), a rotation (build.md 15.1).
	return Vector3(p.x, p.z, -p.y)


static func triangles(primitive: String, bounds: Array) -> Array:
	## Triangles of the shape, each [a, b, c, smooth], in the asset frame, unoriented. Empty when
	## the primitive is not one of the five.
	var lo := Vector3(bounds[0], bounds[1], bounds[2])
	var hi := Vector3(bounds[3], bounds[4], bounds[5])
	var c := (lo + hi) / 2.0
	var r := (hi - lo) / 2.0
	var tris: Array = []
	match primitive:
		"box":
			var v := func(i, j, k): return Vector3(lo.x if i == 0 else hi.x, lo.y if j == 0 else hi.y, lo.z if k == 0 else hi.z)
			for axis in 3:
				for side in 2:
					var q := []
					for a in 2:
						for b in 2:
							var idx := [0, 0, 0]
							idx[axis] = side
							idx[(axis + 1) % 3] = a
							idx[(axis + 2) % 3] = b if a == 0 else 1 - b
							q.append(v.call(idx[0], idx[1], idx[2]))
					tris.append([q[0], q[1], q[2], false])
					tris.append([q[0], q[2], q[3], false])
		"prism":
			var ym := c.y
			var b0 := Vector3(lo.x, lo.y, lo.z)
			var b1 := Vector3(hi.x, lo.y, lo.z)
			var b2 := Vector3(hi.x, hi.y, lo.z)
			var b3 := Vector3(lo.x, hi.y, lo.z)
			var r0 := Vector3(lo.x, ym, hi.z)
			var r1 := Vector3(hi.x, ym, hi.z)
			tris.append_array([[b0, b1, b2, false], [b0, b2, b3, false],
				[b0, b1, r1, false], [b0, r1, r0, false], [b3, b2, r1, false], [b3, r1, r0, false],
				[b0, b3, r0, false], [b1, b2, r1, false]])
		"cylinder", "cone":
			var bottom := _ring(c, r, lo.z)
			var top := _ring(c, r, hi.z)
			var apex := Vector3(c.x, c.y, hi.z)
			for k in SIDES:
				var k1 := (k + 1) % SIDES
				if primitive == "cylinder":
					tris.append([bottom[k], bottom[k1], top[k1], true])
					tris.append([bottom[k], top[k1], top[k], true])
				else:
					tris.append([bottom[k], bottom[k1], apex, true])
			for k in range(1, SIDES - 1):
				tris.append([bottom[0], bottom[k], bottom[k + 1], false])
				if primitive == "cylinder":
					tris.append([top[0], top[k], top[k + 1], false])
		"sphere":
			var rings: Array = []
			for row in range(1, SPHERE_ROWS):
				var phi := deg_to_rad(-90.0 + row * 180.0 / SPHERE_ROWS)
				var rr := Vector3(r.x * cos(phi), r.y * cos(phi), 0)
				rings.append(_ring(c, rr, c.z + r.z * sin(phi)))
			var south := Vector3(c.x, c.y, lo.z)
			var north := Vector3(c.x, c.y, hi.z)
			for k in SIDES:
				var k1 := (k + 1) % SIDES
				tris.append([south, rings[0][k1], rings[0][k], true])
				tris.append([north, rings[-1][k], rings[-1][k1], true])
				for n in rings.size() - 1:
					tris.append([rings[n][k], rings[n][k1], rings[n + 1][k1], true])
					tris.append([rings[n][k], rings[n + 1][k1], rings[n + 1][k], true])
	# Orient: counter-clockwise seen from outside, in the right-handed asset frame.
	for t in tris:
		var n: Vector3 = (t[1] - t[0]).cross(t[2] - t[0])
		var out: Vector3 = (t[0] + t[1] + t[2]) / 3.0 - c
		if n.dot(out) < 0:
			var tmp = t[1]
			t[1] = t[2]
			t[2] = tmp
	return tris


static func mesh(primitive: String, bounds: Array) -> ArrayMesh:
	## The shape as an ArrayMesh in Godot's axes, or null for an unknown primitive.
	var tris := triangles(primitive, bounds)
	if tris.is_empty():
		return null
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for t in tris:
		st.set_smooth_group(0 if t[3] else 0xFFFFFFFF)
		# Godot draws clockwise faces as front faces; the rotation g keeps the turn, so reverse.
		for v in [t[0], t[2], t[1]]:
			st.add_vertex(g(v))
	st.generate_normals()
	return st.commit()


static func points(primitive: String, bounds: Array, scale: float) -> PackedVector3Array:
	## The shape's vertices in Godot's axes, scaled: a convex hull for collision.
	var out := PackedVector3Array()
	var seen := {}
	for t in triangles(primitive, bounds):
		for n in 3:
			var p: Vector3 = g(t[n]) * scale
			if not seen.has(p):
				seen[p] = true
				out.append(p)
	return out


static func _ring(c: Vector3, r: Vector3, z: float) -> Array:
	var out: Array = []
	for k in SIDES:
		var a := deg_to_rad(k * 22.5)
		out.append(Vector3(c.x + r.x * cos(a), c.y + r.y * sin(a), z))
	return out
