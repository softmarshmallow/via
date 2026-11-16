extends SceneTree
## Streaming (spec/build.md 15.1, "Streaming", M6; challenges 42 and 23) against a multi-chunk build:
##   godot --headless --fixed-fps 60 --path engines/godot --script res://tests/test_stream.gd -- \
##         --build DIR [--speed M_PER_S] [--radius M] [--frames] [--log FILE]
## Headless: data and physics only. `--fixed-fps 60` makes every frame 1/60 s of walk time however
## long the machine takes over it: a long walk at a walking pace in seconds, but the worker thread
## works in real time, so it falls behind the walk and the guard makes chunks on the walker's frame
## (stalls) that a real-time walk would not. Without it, each frame is the machine's own time: then
## (and with --frames, one chunk per frame on the walker's thread) no stall is allowed.
##
## 1. Content once: the whole build made at once (Loader.make) and the same build streamed with
##    every chunk loaded hold the same instance names, bodies and generated meshes, each once.
## 2. A long walk: a walker goes round the extent (south-west, north-east, south-east, north-west,
##    south-west, 10 m in from each edge) at --speed (default the sheet walker's 1.4 m/s), one step
##    a frame, standing on the highest collision under it (a ray from 0.5 m over its feet plus the
##    step). It starts with the chunks within the load radius of its start made (a game's loading
##    screen); the counts start after that. Every
##    frame: no chunk is loaded farther than the unload radius, and there is collision under the
##    walker. Each second of walk logs the chunks loaded; the walk reports the most, loads, unloads,
##    stalls (chunks made on the walker's frame because they were not there), the longest plan step
##    in one frame (no teleport: at most speed / 60) and frames with nothing under the walker.
## 3. Load, unload, load: after the walk every chunk is loaded again and check 1 is repeated.
## Prints one line per check and exits 0 when all pass, 1 otherwise. --log writes the per-second
## counts and the totals as JSON.

const Loader := preload("res://addons/via_loader/loader.gd")
const Stream := preload("res://addons/via_loader/stream.gd")

const MARGIN := 10.0
const STEP_UP := 0.5
var failed := 0
var ran := 0


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	ran += 1
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


static func content(root: Node) -> Dictionary:
	## Every instance name in the multimeshes, every instance body, every generated mesh and every
	## chunk node under root, each list sorted; and any node name Godot renamed (holding `@`).
	var out := {"instances": [], "bodies": [], "meshes": [], "chunks": [], "renamed": []}
	var stack: Array[Node] = [root]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		if String(n.name).contains("@"):
			out.renamed.append(String(n.name))
		if n is MultiMeshInstance3D and n.has_meta("names"):
			out.instances.append_array(Array(n.get_meta("names")))
		elif n is StaticBody3D and n.get_parent().name == "bodies":
			out.bodies.append(String(n.name))
		elif n is MeshInstance3D and n.get_parent().name == "meshes":
			out.meshes.append(String(n.name))
		elif n.get_parent() == root and String(n.name).begins_with("c_"):
			out.chunks.append(String(n.name))
		for c in n.get_children():
			stack.append(c)
	for k in ["instances", "bodies", "meshes", "chunks"]:
		out[k].sort()
	return out


static func dupes(list: Array) -> int:
	var d := 0
	for i in range(1, list.size()):
		if list[i] == list[i - 1]:
			d += 1
	return d


func same(what: String, a: Dictionary, b: Dictionary) -> void:
	for k in ["instances", "bodies", "meshes", "chunks"]:
		check("%s: %s as the whole build's, each once" % [what, k], a[k] == b[k] and dupes(b[k]) == 0,
			"%d against %d, %d twice" % [b[k].size(), a[k].size(), dupes(b[k])])
	check("%s: no node renamed" % what, b.renamed.is_empty(), str(b.renamed.slice(0, 5)))


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var dir := ""
	var speed := 1.4
	var radius := Stream.LOAD
	var log_file := ""
	var threaded := true
	var k := 0
	while k < args.size():
		match args[k]:
			"--build": dir = args[k + 1]
			"--speed": speed = float(args[k + 1])
			"--radius": radius = float(args[k + 1])
			"--log": log_file = args[k + 1]
			"--frames": threaded = false; k -= 1
		k += 2
	if dir == "":
		printerr("test_stream: --build DIR is required")
		quit(2)
		return

	# 1. content once
	var whole_loader := Loader.new()
	var whole := whole_loader.make(dir, {})
	var want := content(whole)
	whole.free()
	check("the build has more than one chunk", want.chunks.size() > 1, "%d chunks" % want.chunks.size())

	var loader := Loader.new()
	var root := loader.prepare(dir, {})
	get_root().add_child(root)
	var stream := Stream.new(loader, root, radius)
	stream.threaded = threaded
	check("prepare makes no chunk", content(root).chunks.is_empty())
	stream.fill(Vector2.ZERO, INF)
	same("streamed, every chunk loaded", want, content(root))

	# 2. the long walk: unload everything first (a walker far off), then walk the round
	var ext: Array = root.get_meta("frame").extent
	stream.update(Vector2(1e9, 1e9))
	check("a walker far off unloads every chunk", stream.loaded.is_empty(), "%d left" % stream.loaded.size())
	var a := Vector2(float(ext[0]) + MARGIN, float(ext[1]) + MARGIN)
	var b := Vector2(float(ext[2]) - MARGIN, float(ext[3]) - MARGIN)
	var route: Array[Vector2] = [a, b, Vector2(b.x, a.y), Vector2(a.x, b.y), a]
	var dt := 1.0 / 60.0
	var p := route[0]
	var next := 1
	var feet := loader.ground_at(p.x, p.y) if loader.has_ground() else 0.0
	var space := root.get_world_3d().direct_space_state
	var per_second: Array[int] = []
	var most := 0
	var past := 0
	var far_wrong := 0
	var nothing_under := 0
	var snapped := 0
	var longest := 0.0
	var climb := 0.0
	var frames := 0
	var walked_s := 0.0
	var longest_dt := 0.0
	var longest_at := 0
	var work: Array[int] = []
	var t0 := Time.get_ticks_msec()
	# A walk starts in a loaded world: what lies within the load radius of its start is made
	# first, as a game makes it behind its loading screen; the counts start after that.
	stream.fill(p, stream.load_radius)
	await process_frame   # the loading screen's frame: its time is not a step of the walk
	stream.loads = 0
	stream.unloads = 0
	stream.stalls = 0
	var was := p
	while next < route.size():
		stream.update(p)
		work.append(stream.work_usec)
		most = maxi(most, stream.loaded.size())
		for n in stream.loaded:
			if stream.distance(n, p) > stream.unload_radius:
				past += 1
		if frames % 60 == 0 and loader.has_ground():
			for n in stream.chunk_count():
				if stream.far_shown(n) == stream.loaded.has(n):
					far_wrong += 1
		# stand on the highest collision under the walker: a ray from STEP_UP over its feet, plus
		# the frame's plan step (a slope up to 45 degrees), so a fast frame up a hill starts above it
		var rise := STEP_UP + p.distance_to(was)
		var q := PhysicsRayQueryParameters3D.create(Loader.g(p.x, p.y, feet + rise), Loader.g(p.x, p.y, feet - 1000.0))
		var hit := space.intersect_ray(q)
		if hit.is_empty():
			# under a face steeper than that (vale's bluffs): the collision is there if a ray
			# from high above meets it; the follower is put back on top
			q.from = Loader.g(p.x, p.y, feet + 10000.0)
			hit = space.intersect_ray(q)
			if not hit.is_empty():
				snapped += 1
		if hit.is_empty():
			nothing_under += 1
		else:
			climb = maxf(climb, absf(hit.position.y - feet))
			feet = hit.position.y
		if frames % 60 == 0:
			per_second.append(stream.loaded.size())
		frames += 1
		await process_frame
		# the frame's own time: 1/60 s under --fixed-fps 60, else the machine's
		dt = root.get_process_delta_time()
		walked_s += dt
		if dt > longest_dt:
			longest_dt = dt
			longest_at = frames
		var left := speed * dt
		was = p
		while left > 0.0 and next < route.size():
			var d := p.distance_to(route[next])
			if d <= left:
				left -= d
				p = route[next]
				next += 1
			else:
				p += (route[next] - p) / d * left
				left = 0.0
		longest = maxf(longest, p.distance_to(was))
	stream.end()
	var real_s := (Time.get_ticks_msec() - t0) / 1000.0
	work.sort()
	check("no chunk loaded past the unload radius on any frame", past == 0,
		"%d chunk-frames past %.0f m" % [past, stream.unload_radius])
	if loader.has_ground():
		check("far ground: one per chunk", stream.far_count() == stream.chunk_count(),
			"%d of %d" % [stream.far_count(), stream.chunk_count()])
		check("far ground shown exactly where the chunk is not loaded, once a second", far_wrong == 0,
			"%d chunk-seconds wrong" % far_wrong)
	check("collision under the walker on every frame", nothing_under == 0, "%d of %d frames with none" % [nothing_under, frames])
	# 1 mm for single-precision plan points a few kilometres out
	check("no teleport: no plan step longer than speed times the longest frame", longest <= speed * longest_dt + 0.001,
		"%.4f m against %.4f (frame %.1f ms)" % [longest, speed * longest_dt, longest_dt * 1000.0])
	# --fixed-fps 60: the walk's time ran well ahead of the machine's
	var accelerated := real_s < walked_s * 0.5
	if not accelerated or not threaded:
		check("no stall: every chunk the walker reached was loaded before it got there", stream.stalls == 0,
			"%d stalls" % stream.stalls)
	check("chunks unload as the walk leaves them", stream.unloads > 0,
		"most %d of %d loaded, %d loads, %d unloads" % [most, stream.chunk_count(), stream.loads, stream.unloads])
	var summary := {
		"build": dir.get_file(), "accelerated": accelerated, "speed": speed, "threaded": threaded, "load_radius": stream.load_radius,
		"unload_radius": stream.unload_radius, "guard": stream.guard, "chunks": stream.chunk_count(),
		"walk_s": walked_s, "real_s": real_s, "frames": frames, "longest_frame_ms": longest_dt * 1000.0, "longest_frame_at": longest_at, "most_loaded": most,
		"loads": stream.loads, "unloads": stream.unloads, "stalls": stream.stalls,
		"longest_step_m": longest, "largest_height_change_m": climb, "frames_nothing_under": nothing_under, "frames_put_on_top": snapped,
		"work_ms": {"median": work[work.size() / 2] / 1000.0, "p99": work[int(work.size() * 0.99)] / 1000.0,
			"max": work[-1] / 1000.0},
		"loaded_per_second": per_second,
	}
	print("walk: ", JSON.stringify(summary.duplicate().merged({"loaded_per_second": "%d values" % per_second.size()}, true)))

	# 3. load, unload, load
	stream.fill(Vector2.ZERO, INF)
	same("after the walk's unloads, every chunk loaded again", want, content(root))
	stream.update(Vector2(1e9, 1e9))
	check("and unloaded again, nothing is left", content(root).chunks.is_empty() and content(root).meshes.is_empty())

	if log_file != "":
		var f := FileAccess.open(log_file, FileAccess.WRITE)
		f.store_string(JSON.stringify(summary, "  ", true))
		f.close()
	root.free()
	print("%s: %d check(s) failed" % ["PASS" if failed == 0 else "FAIL", failed])
	quit(0 if failed == 0 else 1)
