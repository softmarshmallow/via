extends RefCounted
## The sheet tool's walker (spec/build.md 15.1): "on foot" as the engine has it, not only as an eye
## sees it. A capsule CharacterBody3D, RADIUS by HEIGHT, walks from a bookmark's point toward the
## place it looks at (its `toward`), straight in plan at SPEED, under gravity, colliding with what
## the loader made: the ground's height map and every instance body (collision_of). It is scripted,
## not driven: each step sets the velocity and calls move_and_slide once, DT apart, so a walk is the
## same on every run on one machine.
##
## It stops at the first of:
##   arrived    within ARRIVE metres of the target in plan, or against the target's own body (a walk
##              to a house ends at its wall; a set's items are its placement's body);
##   blocked    less than STALL_PROGRESS metres nearer in STALL_TIME seconds: `by` names what it
##              pressed against last, an instance or `ground` (with the slope, steeper than
##              MAX_SLOPE degrees);
##   fell       it dropped more than FALL metres before landing again, beyond what the ground
##              itself falls under its path over that distance (a ledge), or below the ground;
##              a descent through the air down ground with no ledge, too steep to walk (steeper
##              than MAX_SLOPE degrees), is `blocked` by `ground` with that slope, as going up is;
##   timed_out  three times the time the straight line needs, plus 5 s.
## Water deeper than WADE metres stops it at the shore: `blocked` by `water`, or `arrived` (by
## `water`) when the place it walks to stands in such water itself, as a walk to a house ends at
## its wall. Depth is taken against the feet, not the ground: the water's surface (ground plus
## depth_at) less the feet, so a walker on a deck or a floor over deep water walks on; on the
## ground the two are the same.
## Along the way it records the deepest water it waded (depth_at) and every solid instance whose
## footprint it walked through because the asset collides as nothing (solid_without_body): the core
## keeps things off such a footprint, so walking through one says the pack has not given it a body.

const RADIUS := 0.3
const HEIGHT := 1.8
const EYE := 1.7
const SPEED := 1.4
const DT := 1.0 / 60.0
const GRAVITY := 9.8
const MAX_SLOPE := 40.0
const SNAP := 0.5
const ARRIVE := 1.0
const STALL_TIME := 1.0
const STALL_PROGRESS := 0.1
const FALL := 1.0
const TRAIL_STEP := 0.5
const LIFT := 0.05
## The deepest water a person wades, metres; deeper water stops the walk at the shore.
const WADE := 0.7
## The eye never stands lower than this above the water surface, metres.
const ABOVE_WATER := 0.2
## The eye view (eye()): BACK metres before the stop; turned round when the extent ends EDGE ahead.
const BACK := 6.0
const EDGE := 25.0
## The highest edge of a kind surface's mesh the walker steps up onto, metres: a stair's riser or a
## sill. Only onto a generated mesh (its body carries metadata `surface`), never onto the ground or
## an instance, so a slope still blocks as ground and a wall as itself.
const STEP := 0.3
## Following a path (`along`): a point counts as reached within WAYPOINT metres of it.
const WAYPOINT := 0.3


static func wading(loader, x: float, y: float, feet: float) -> float:
	## How deep a walker whose feet are at `feet` (world metres) stands in water at (x, y): the
	## water's surface less the feet, 0 where it is dry or the feet are above the surface.
	var d: float = loader.depth_at(x, y)
	if d <= 0.0:
		return 0.0
	return maxf(0.0, loader.ground_at(x, y) + d - feet)


static func under(root: Node3D, top: Vector3, depth: float, exclude: Array[RID] = []) -> Dictionary:
	## The first collision straight down from `top` (a Godot point) within `depth` metres:
	## { "y": its height, "on": what it is } or {} for nothing. `on` is `ground`, a generated mesh's
	## name (build.md 10) or an instance's name.
	var q := PhysicsRayQueryParameters3D.create(top, top - Vector3(0, depth, 0))
	q.exclude = exclude
	var hit := root.get_world_3d().direct_space_state.intersect_ray(q)
	if hit.is_empty():
		return {}
	return {"y": hit.position.y, "on": surface_name(hit.collider)}


static func surface_name(collider: Object) -> String:
	var n := collider as Node
	if n.has_meta("surface"):
		return String(n.get_meta("surface"))
	if n.name == "ground_body":
		return "ground"
	return String(n.name).replace("-", ".")


static func _step_up(body: CharacterBody3D, facing: Vector2) -> bool:
	## Onto a ledge at most STEP above the feet: up STEP, RADIUS forward, then down onto what is
	## there, which must be walkable. False, and nothing moved, when any part of that is blocked.
	var up := Vector3(0, STEP, 0)
	var fwd := Vector3(facing.x, 0, -facing.y) * RADIUS
	if body.test_move(body.global_transform, up):
		return false
	var t := body.global_transform.translated(up)
	if body.test_move(t, fwd):
		return false
	t = t.translated(fwd)
	var land := KinematicCollision3D.new()
	if not body.test_move(t, Vector3(0, -(STEP + LIFT), 0), land):
		return false
	if rad_to_deg(land.get_normal().angle_to(Vector3.UP)) > MAX_SLOPE:
		return false
	if not (land.get_collider() as Node).has_meta("surface"):
		return false   # onto a generated mesh only: never onto the ground or an instance
	body.global_position = t.origin + land.get_travel()
	return true


static func g(x: float, y: float, z: float) -> Vector3:
	return Vector3(x, z, -y)


static func steepest_between(loader, a: Vector2, b: Vector2) -> float:
	## The steepest ground in degrees along the plan segment a-b, every quarter metre, as the
	## ground mesh draws it (loader.ground_at); MAX_SLOPE when there is no ground.
	if not loader.has_ground():
		return MAX_SLOPE
	var n := maxi(1, ceili(a.distance_to(b) / 0.25))
	var most := 0.0
	var prev: Vector2 = a
	for k in range(1, n + 1):
		var q := a.lerp(b, float(k) / n)
		var run := q.distance_to(prev)
		if run > 0.0:
			var rise := absf(loader.ground_at(q.x, q.y) - loader.ground_at(prev.x, prev.y))
			most = maxf(most, rad_to_deg(atan2(rise, run)))
		prev = q
	return most


static func plan(v: Vector3) -> Vector2:
	## Godot point to plan metres (x east, y north).
	return Vector2(v.x, -v.z)


## Walks once. `root` is the loaded build in its viewport (its bodies already in the physics
## space); `loader` the Loader that made it; `from` the eye's world point (x, y, z: a bookmark's
## `at` raised by its eye height) and `to` the target's; `target` the name of what `to` is; `along`
## the plan points [x, y] of a path to follow first (a bookmark's `along`, build.md 15.1), or none:
## it aims at each point in turn, taking the next once within WAYPOINT of it or nearer the one
## after than that point is, and only after the last at `to`; it arrives only once past them all,
## and its progress (for `blocked`) and time limit are measured along that route. The
## walker stands on the highest collision straight below the eye (the terrain, a kind surface's
## mesh or an instance), then follows whatever it walks onto. Returns the report sheet.json
## carries, plus "trail": PackedVector3Array of Godot points every TRAIL_STEP metres and "facing":
## the last plan direction. With `stream` (stream.gd), the chunks round the walker are loaded as it
## goes, the chunk under it before each step, in lockstep (build.md 15.1, streaming), so a streamed
## walk ends in the same place on every run.
static func walk(root: Node3D, loader, from: Vector3, to: Vector3, target: String, unbodied: Array, along: Array = [], stream = null) -> Dictionary:
	var was_lockstep := false
	if stream != null:
		was_lockstep = stream.lockstep
		stream.lockstep = true   # the same bodies in the same order on every run (stream.gd)
		stream.update(Vector2(from.x, from.y))
	await root.get_tree().physics_frame
	var floor_y: float = loader.ground_min() if loader.has_ground() else from.z - 1000.0
	var start := under(root, g(from.x, from.y, from.z), from.z - floor_y + 1000.0)
	var start_z: float = start.y if not start.is_empty() else from.z - EYE
	var on_from = start.get("on")
	var body := CharacterBody3D.new()
	body.name = "walker"
	var cs := CollisionShape3D.new()
	var cap := CapsuleShape3D.new()
	cap.radius = RADIUS
	cap.height = HEIGHT
	cs.shape = cap
	body.add_child(cs)
	body.floor_max_angle = deg_to_rad(MAX_SLOPE)
	body.floor_snap_length = SNAP
	body.floor_stop_on_slope = true
	body.up_direction = Vector3.UP
	body.position = g(from.x, from.y, start_z + HEIGHT / 2.0 + LIFT)
	root.add_child(body)
	await root.get_tree().physics_frame
	await root.get_tree().physics_frame

	var goal := Vector2(to.x, to.y)
	var ways: Array[Vector2] = []
	for q in along:
		ways.append(Vector2(float(q[0]), float(q[1])))
	# tail[i]: the route's length from ways[i] through the rest to the goal; tail[n] = 0
	var tail: Array[float] = []
	tail.resize(ways.size() + 1)
	tail[ways.size()] = 0.0
	for i in range(ways.size() - 1, -1, -1):
		tail[i] = tail[i + 1] + ways[i].distance_to(ways[i + 1] if i + 1 < ways.size() else goal)
	var next := 0
	var route := Vector2(from.x, from.y).distance_to(ways[0] if not ways.is_empty() else goal) + tail[0]
	var limit := int((route / SPEED * 3.0 + 5.0) / DT)
	var stall := int(STALL_TIME / DT)
	var history: Array[float] = []
	var trail := PackedVector3Array()
	var walked := 0.0
	var last := plan(body.position)
	var trail_at := 0.0
	trail.append(body.position - Vector3(0, HEIGHT / 2.0, 0))
	var vz := 0.0
	var airborne := false
	var air_from := 0.0
	var air_at := Vector2.ZERO
	var deepest := {"depth": 0.0, "at": null}
	var through := {}
	var facing := (goal - last).normalized()
	var report := {"outcome": "timed_out", "by": null, "slope": null, "drop": null, "at_target": null}
	var ground_min := INF
	if loader.has_ground():
		ground_min = loader.ground_min()
	var target_wet: bool = wading(loader, to.x, to.y, to.z) > WADE
	for step in limit:
		var p := plan(body.position)
		if stream != null:
			stream.update(p)
		while next < ways.size() and (p.distance_to(ways[next]) <= WAYPOINT or (next + 1 < ways.size()
				and p.distance_to(ways[next + 1]) < ways[next].distance_to(ways[next + 1]))):
			next += 1
		var aim: Vector2 = ways[next] if next < ways.size() else goal
		var left := p.distance_to(goal)
		# Not in the air: a walker falling past its target has not arrived (build.md 15.1).
		if next >= ways.size() and left <= ARRIVE and not airborne:
			report.outcome = "arrived"
			break
		var to_aim := maxf(p.distance_to(aim), 1e-6)
		facing = (aim - p) / to_aim
		body.velocity = Vector3(facing.x, 0, -facing.y) * SPEED + Vector3(0, vz, 0)
		body.move_and_slide()
		if body.is_on_floor():
			# Against the edge of a kind surface's mesh within STEP of the feet: step onto it.
			var stepped := false
			for k in body.get_slide_collision_count():
				var c := body.get_slide_collision(k)
				if (c.get_collider() as Node).has_meta("surface") and rad_to_deg(c.get_normal().angle_to(Vector3.UP)) > MAX_SLOPE:
					if _step_up(body, facing):
						stepped = true
						break
			# Pressed into the crease where a sloping tread meets a floor's edge, the capsule
			# reports only the tread and does not move: try the step there too (onto a mesh only).
			if not stepped and plan(body.position).distance_to(p) < SPEED * DT * 0.25:
				_step_up(body, facing)
		var feet := body.position.y - HEIGHT / 2.0
		if body.is_on_floor():
			vz = 0.0
			if airborne:
				airborne = false
				var landed := plan(body.position)
				# The height lost through the air less what the ground falls under the path: a
				# ledge, not a slope (build.md 15.1).
				var ground_fall := 0.0
				if loader.has_ground():
					ground_fall = loader.ground_at(air_at.x, air_at.y) - loader.ground_at(landed.x, landed.y)
				var ledge := (air_from - feet) - ground_fall
				if ledge > FALL:
					report.outcome = "fell"
					report.drop = ledge
					break
				if air_from - feet > FALL:
					# Down ground too steep to walk, with no ledge: blocked by the ground.
					report.outcome = "blocked"
					report.by = "ground"
					report.slope = steepest_between(loader, air_at, landed)
					break
		else:
			vz -= GRAVITY * DT
			if not airborne:
				airborne = true
				air_from = feet
				air_at = plan(body.position)
			if feet < ground_min - FALL:
				report.outcome = "fell"
				report.drop = air_from - feet
				break
		var q := plan(body.position)
		walked += q.distance_to(p)
		trail_at += q.distance_to(p)
		if trail_at >= TRAIL_STEP:
			trail_at = 0.0
			trail.append(Vector3(body.position.x, feet, body.position.z))
		var d: float = wading(loader, q.x, q.y, feet)
		# In the air it is falling, not wading: off a deck into a river is `fell` once it lands
		# (build.md 15.1), never `arrived` because the target stands in the water.
		if d > WADE and not airborne:
			# Too deep to wade: it stops at the shore, the step into it taken back.
			body.position = Vector3(body.position.x - (q.x - p.x), body.position.y, body.position.z + (q.y - p.y))
			walked -= q.distance_to(p)
			report.outcome = "arrived" if target_wet else "blocked"
			report.by = "water"
			report.at_target = "water" if target_wet else null
			report.slope = null
			break
		if d > deepest.depth:
			deepest = {"depth": d, "at": [q.x, q.y]}
		for u in unbodied:
			if not through.has(u.name) and Geometry2D.is_point_in_polygon(q, u.plan):
				through[u.name] = true
		# what it pressed against this step: the most upright surface it touched
		var hit = null
		var steepest := -1.0
		for k in body.get_slide_collision_count():
			var c := body.get_slide_collision(k)
			var angle := rad_to_deg(c.get_normal().angle_to(Vector3.UP))
			if angle > steepest:
				steepest = angle
				hit = c
		if hit != null and steepest > MAX_SLOPE:
			var who := String((hit.get_collider() as Node).name)
			if who == "ground_body":
				report.by = "ground"
				report.slope = steepest
			else:
				report.by = who.replace("-", ".")
				report.slope = null
				# its own body: the target itself, the thing a mount or end of it names
				# (`cottage.door`), or an item of a set placement it names (`desk.chair`; an
				# instance, not a kind surface's mesh, whose edge is stepped onto)
				var item: bool = report.by.begins_with(target + ".") and not (hit.get_collider() as Node).has_meta("surface")
				if report.by == target or target.begins_with(report.by + ".") or item:
					report.outcome = "arrived"
					report.at_target = report.by
					break
		history.append(to_aim + tail[next])
		if history.size() > stall:
			var then: float = history.pop_front()
			if then - (q.distance_to(aim) + tail[next]) < STALL_PROGRESS:
				report.outcome = "blocked"
				break
	var end := body.position - Vector3(0, HEIGHT / 2.0, 0)
	trail.append(end)
	var below := under(root, body.position, HEIGHT / 2.0 + SNAP, [body.get_rid()])
	root.remove_child(body)
	body.free()
	if stream != null:
		stream.lockstep = was_lockstep
	if report.outcome != "blocked":
		report.by = report.at_target   # arrived against its target's body, or null
		report.slope = null
	var names := through.keys()
	names.sort()
	return {
		"by": report.by,
		"deepest": deepest if deepest.at != null else null,
		"drop": report.drop,
		"from": [from.x, from.y, start_z],
		"left": plan(end).distance_to(goal),
		"on": {"from": on_from, "stop": below.get("on")},
		"outcome": report.outcome,
		"slope": report.slope,
		"stop": [end.x, -end.z, end.y],
		"through": names,
		"toward": target,
		"walked": walked,
		"trail": trail,
		"facing": facing,
	}


static func summary(r) -> String:
	## One line for a sheet label.
	if r == null:
		return "absent"
	var s := "on %s, %s after %.1f m" % [r.on.from if r.on.from != null else "nothing", r.outcome, r.walked]
	if r.on.stop != r.on.from and r.on.stop != null:
		s += " onto %s" % r.on.stop
	match r.outcome:
		"blocked":
			s += " by %s" % (r.by if r.by != null else "?")
			if r.slope != null:
				s += " (%.0f deg)" % r.slope
		"arrived":
			if r.by != null:
				s += " at %s" % r.by
		"fell":
			s += ", %.1f m drop" % r.drop
	if r.deepest != null and r.deepest.depth >= 0.05:
		s += ", wades %.2f m" % r.deepest.depth
	if not r.through.is_empty():
		s += ", through %d without collision" % r.through.size()
	return s


## The walk drawn on the ground: a ribbon RIBBON metres wide just above each trail point, unshaded,
## in the outcome's colour, with a disc where it stopped.
const RIBBON := 0.5
const COLOURS := {"arrived": Color(1.0, 0.85, 0.1), "blocked": Color(1.0, 0.3, 0.1),
	"fell": Color(1.0, 0.1, 0.1), "timed_out": Color(0.8, 0.8, 0.8)}


static func trail_node(r: Dictionary) -> Node3D:
	var holder := Node3D.new()
	holder.name = "walk"
	var trail: PackedVector3Array = r.trail
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = COLOURS.get(r.outcome, Color.WHITE)
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	if trail.size() >= 2:
		var verts := PackedVector3Array()
		for k in trail.size():
			var a := trail[maxi(k - 1, 0)]
			var b := trail[mini(k + 1, trail.size() - 1)]
			var d := Vector3(b.x - a.x, 0, b.z - a.z).normalized()
			var side := Vector3(-d.z, 0, d.x) * RIBBON / 2.0
			var p := trail[k] + Vector3(0, 0.15, 0)
			verts.append(p - side)
			verts.append(p + side)
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = verts
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLE_STRIP, arrays)
		var mi := MeshInstance3D.new()
		mi.mesh = mesh
		mi.material_override = mat
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		holder.add_child(mi)
	var disc := MeshInstance3D.new()
	var cyl := CylinderMesh.new()
	cyl.top_radius = 1.0
	cyl.bottom_radius = 1.0
	cyl.height = 0.3
	disc.mesh = cyl
	disc.material_override = mat
	disc.position = trail[trail.size() - 1] + Vector3(0, 0.15, 0)
	disc.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	holder.add_child(disc)
	return holder


static func overhead(r: Dictionary, aspect: float) -> Camera3D:
	## An orthographic camera straight down over the walk, MARGIN metres beyond it on each side.
	const MARGIN := 12.0
	var trail: PackedVector3Array = r.trail
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	var top := -INF
	for p in trail:
		lo = Vector2(minf(lo.x, p.x), minf(lo.y, p.z))
		hi = Vector2(maxf(hi.x, p.x), maxf(hi.y, p.z))
		top = maxf(top, p.y)
	var c := (lo + hi) / 2.0
	var span := hi - lo + Vector2(MARGIN, MARGIN) * 2.0
	var cam := Camera3D.new()
	cam.projection = Camera3D.PROJECTION_ORTHOGONAL
	cam.size = maxf(span.y, span.x / aspect)   # keep_height: size is the vertical extent
	cam.near = 0.5
	cam.far = 400.0
	cam.transform = Transform3D(Basis.looking_at(Vector3.DOWN, Vector3.FORWARD), Vector3(c.x, top + 150.0, c.y))
	return cam


static func eye(r: Dictionary, extent: Array, loader = null) -> Camera3D:
	## The walker's eye near where it stopped: EYE above the trail BACK metres before the stop,
	## looking along the walk and a little down, so what stopped it, or what it arrived at, is in
	## the frame. Where the world's extent ends within EDGE metres ahead (a walk to a lane's end at
	## the border), it turns round at the stop and looks back the way it came.
	var trail: PackedVector3Array = r.trail
	var f: Vector2 = r.facing
	var stop := trail[trail.size() - 1]
	var from := stop
	var k := trail.size() - 1
	while k > 0 and plan(trail[k]).distance_to(plan(stop)) < BACK:
		k -= 1
	from = trail[k]
	var ahead := plan(stop) + f * EDGE
	if ahead.x < float(extent[0]) or ahead.y < float(extent[1]) or ahead.x > float(extent[2]) or ahead.y > float(extent[3]):
		from = stop
		f = -f
	var at := from + Vector3(0, EYE, 0)
	if loader != null:
		# Never under the water: at least ABOVE_WATER over its surface where it stands in water.
		var q := plan(from)
		var depth: float = loader.depth_at(q.x, q.y)
		if depth > 0.0:
			var surface: float = loader.ground_at(q.x, q.y) + depth
			at.y = maxf(at.y, surface + ABOVE_WATER)
	var look := at + Vector3(f.x, 0, -f.y) * 10.0 + Vector3(0, -1.5, 0)
	var cam := Camera3D.new()
	cam.near = 0.05
	cam.transform = Transform3D(Basis.looking_at(look - at, Vector3.UP), at)
	return cam
