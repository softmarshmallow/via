extends RefCounted
## Streams a build's chunks around a walker (spec/build.md 15.1, "Streaming", M6). The loader's
## `prepare` makes the root with what chunks share; this makes each chunk (ground, water, weights,
## instances and their bodies) and each generated mesh as the walker nears it, and frees them as it
## leaves, from the manifest's chunk list and `meshes` list alone.
##
##   var loader := Loader.new()
##   var root := loader.prepare(dir, opts)
##   add_child(root)
##   var stream := Stream.new(loader, root)
##   stream.update(Vector2(x, y))   # every frame, with the walker's plan point
##
## - Load: every chunk whose plan rectangle lies within `load_radius` of the walker, nearest first,
##   one at a time, made on a worker thread (`threaded`) and added to the tree on the walker's
##   frame; or, not threaded, one chunk per update on the walker's own thread.
## - Unload: a chunk farther than `unload_radius` (the load radius plus one chunk side, so a walker
##   on a chunk's edge does not load and unload it every step) is freed.
## - Guard: a chunk within `guard` of the walker that is not loaded is made at once on the walker's
##   thread (waiting for the worker first), so the walker never stands over a chunk that is not
##   there. Each such load is a `stall`, counted: a stall is a frame the walk waited for a chunk.
## - A generated mesh belongs to the chunks whose `meshes` list names it (build.md 5, M6); in a
##   build whose chunks hold no such list, to every chunk its plan bounds meet (Loader.mesh_rect;
##   every chunk when the file cannot say). It is loaded while any of them is, once.
## - Far ground: every chunk has a coarse ground (Loader.make_far_ground: at most 8 quads a side,
##   coloured by its weights, not collided, all chunks in one mesh and one draw call), made once
##   here and shown exactly while its chunk is not loaded (one texel per chunk of the mesh's
##   `loaded` texture), so past the load radius the view still has ground to the camera's far
##   distance instead of a void (M6 slice 1 review on foot: hill towns and far banks vanished past
##   96 m).

## Ours: 96 m is one and a half default chunks (64 m, build.md 5), so the chunks a walker may see
## within 96 m are there; GUARD is a walker's reach in a few frames at any pace a game walks.
const LOAD := 96.0
const GUARD := 4.0

var loader
var root: Node3D
var load_radius := LOAD
var unload_radius := LOAD
var guard := GUARD
var threaded := true
## chunk index -> its node, while loaded
var loaded := {}
var loads := 0
var unloads := 0
var stalls := 0
## Microseconds of this thread's time the last update took, and the most any update took.
var work_usec := 0
var work_max_usec := 0

var _rects: Array[Rect2] = []
var _side := 0.0
var _chunk_meshes: Array = []   # per chunk: indices into the mesh list
var _meshes: Array = []
var _mesh_nodes := {}   # mesh index -> MeshInstance3D
var _mesh_refs := {}    # mesh index -> loaded chunks that hold it
var _holder: Node3D
var _far: MeshInstance3D
var _far_mask: Image            # one R8 texel per chunk, at `[cx, cy]`: 255 while it is loaded
var _far_tex: ImageTexture
var _ats: Array[Vector2i] = []  # chunk index -> its `at`
var _task := -1
var _task_n := -1
var _result := {}


func _init(the_loader, the_root: Node3D, radius := LOAD) -> void:
	loader = the_loader
	root = the_root
	var chunks: Array = loader.chunk_list()
	for n in chunks.size():
		var r: Rect2 = loader.chunk_rect(n)
		_rects.append(r)
		_side = maxf(_side, maxf(r.size.x, r.size.y))
		_chunk_meshes.append([])
	set_radius(radius)
	var size := Vector2i(1, 1)
	for c in chunks:
		var at: Array = c.get("at", [0, 0])
		_ats.append(Vector2i(int(at[0]), int(at[1])))
		size = size.max(_ats[-1] + Vector2i(1, 1))
	_far = loader.make_far_ground()
	if _far != null:
		_far_mask = Image.create_empty(size.x, size.y, false, Image.FORMAT_R8)
		_far_tex = ImageTexture.create_from_image(_far_mask)
		_far.material_override.set_shader_parameter("loaded", _far_tex)
		root.add_child(_far)
	_meshes = loader.mesh_list()
	var listed := chunks.all(func(c): return c.get("meshes") is Array)
	var index := {}
	for i in _meshes.size():
		if _meshes[i] is Dictionary:
			index[_meshes[i].get("name")] = i
	for i in _meshes.size():
		if listed:
			break
		var mr: Rect2 = loader.mesh_rect(_meshes[i])
		for n in _rects.size():
			if mr.size.x < 0.0 or _rects[n].intersects(mr, true):
				_chunk_meshes[n].append(i)
	for n in _rects.size():
		if listed:
			for name in chunks[n].meshes:
				if index.has(name):
					_chunk_meshes[n].append(index[name])
	if not _meshes.is_empty():
		_holder = Node3D.new()
		_holder.name = "meshes"
		root.add_child(_holder)


func set_radius(radius: float) -> void:
	load_radius = radius
	unload_radius = radius + _side


func chunk_count() -> int:
	return _rects.size()


func distance(n: int, p: Vector2) -> float:
	## Plan metres from p to chunk n's rectangle (0 inside it).
	return p.distance_to(p.clamp(_rects[n].position, _rects[n].end))


func far_shown(n: int) -> bool:
	## Whether chunk n's far ground is shown (false without a far ground).
	return _far != null and _far_mask.get_pixelv(_ats[n]).r < 0.5


func far_count() -> int:
	## Chunks the far ground covers: every chunk, or none without a surface field.
	return _ats.size() if _far != null else 0


func _far_mark(n: int, is_loaded: bool) -> void:
	if _far != null:
		_far_mask.set_pixelv(_ats[n], Color(1.0 if is_loaded else 0.0, 0, 0))
		_far_tex.update(_far_mask)


func meshes_of(n: int) -> Array:
	return _chunk_meshes[n]


func update(p: Vector2) -> void:
	var t0 := Time.get_ticks_usec()
	if _task >= 0 and WorkerThreadPool.is_task_completed(_task):
		_finish()
	for n in loaded.keys():
		if distance(n, p) > unload_radius:
			_unload(n)
	for n in _rects.size():
		if not loaded.has(n) and distance(n, p) <= guard:
			stalls += 1
			_now(n)
	if _task < 0:
		var best := -1
		var best_d := INF
		for n in _rects.size():
			var d := distance(n, p)
			if not loaded.has(n) and d <= load_radius and d < best_d:
				best = n
				best_d = d
		if best >= 0:
			if threaded:
				_task_n = best
				_result = {}
				_task = WorkerThreadPool.add_task(_build.bind(best, _wanted(best)), false, "via chunk")
			else:
				_attach(best, _made(best, _wanted(best)))
	work_usec = Time.get_ticks_usec() - t0
	work_max_usec = maxi(work_max_usec, work_usec)


func fill(p: Vector2, radius: float) -> void:
	## Every chunk within `radius` of p made now, on this thread: for a still picture.
	for n in _rects.size():
		if not loaded.has(n) and distance(n, p) <= radius:
			_now(n)


func end() -> void:
	## Waits for the worker; what it made and was not added is freed.
	if _task >= 0:
		WorkerThreadPool.wait_for_task_completion(_task)
		_task = -1
		if _result.has("chunk"):
			_result.chunk.free()
			for m in _result.meshes.values():
				m.free()
		_result = {}


func _now(n: int) -> void:
	## Chunk n loaded before returning: the worker's chunk is taken when it is n, else waited for
	## so the loader is never used by two threads at once.
	if _task >= 0:
		_finish()
	if not loaded.has(n):
		_attach(n, _made(n, _wanted(n)))


func _wanted(n: int) -> Array:
	return _chunk_meshes[n].filter(func(i): return not _mesh_nodes.has(i))


func _build(n: int, wanted: Array) -> void:
	_result = _made(n, wanted)


func _made(n: int, wanted: Array) -> Dictionary:
	var made := {}
	for i in wanted:
		var m = loader.make_mesh(_meshes[i])
		if m != null:
			made[i] = m
	return {"chunk": loader.make_chunk(n), "meshes": made}


func _finish() -> void:
	WorkerThreadPool.wait_for_task_completion(_task)
	_task = -1
	var res := _result
	_result = {}
	_attach(_task_n, res)


func _attach(n: int, res: Dictionary) -> void:
	root.add_child(res.chunk)
	loaded[n] = res.chunk
	_far_mark(n, true)
	loads += 1
	for i in res.meshes:
		_holder.add_child(res.meshes[i])
		_mesh_nodes[i] = res.meshes[i]
	for i in _chunk_meshes[n]:
		_mesh_refs[i] = _mesh_refs.get(i, 0) + 1


func _unload(n: int) -> void:
	var node: Node3D = loaded[n]
	loaded.erase(n)
	root.remove_child(node)
	node.free()
	unloads += 1
	_far_mark(n, false)
	for i in _chunk_meshes[n]:
		_mesh_refs[i] = _mesh_refs.get(i, 1) - 1
		if _mesh_refs[i] <= 0 and _mesh_nodes.has(i):
			var m: Node = _mesh_nodes[i]
			_mesh_nodes.erase(i)
			_mesh_refs.erase(i)
			_holder.remove_child(m)
			m.free()
