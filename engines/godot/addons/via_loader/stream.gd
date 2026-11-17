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
## - Lockstep (`lockstep`): the worker's chunk is waited for on the next update, for a scripted
##   walk that must end in the same place on every run.
## - Guard: a chunk within `guard` of the walker that is not loaded is made at once on the walker's
##   thread (waiting for the worker first), so the walker never stands over a chunk that is not
##   there. Each such load is a `stall`, counted: a stall is a frame the walk waited for a chunk.
## - A generated mesh belongs to the chunks whose `meshes` list names it (build.md 5, M6); in a
##   build whose chunks hold no such list, to every chunk its plan bounds meet (Loader.mesh_rect;
##   every chunk when the file cannot say). It is loaded while any of them is, once.
## - Far ground: every chunk has a coarse ground (Loader.make_far_ground: posts at most 8 m apart,
##   coloured by its weights, not collided, all chunks in one mesh and one draw call), made once
##   here and shown exactly while its chunk is not loaded (one texel per chunk of the mesh's
##   `loaded` texture), so past the load radius the view still has ground to the camera's far
##   distance instead of a void (M6 slice 1 review on foot: hill towns and far banks vanished past
##   96 m).
## - Far instances: every chunk's instances as boxes of their assets' bounds in one colour each
##   (Loader.make_far_instances), one mesh and one draw call for all chunks, shown with the far
##   ground's texture exactly while the chunk is not loaded, so far houses stand on far ground
##   (M6 slice 2). `far_instances = false` before making the streamer leaves them out.
## - Far water and skirts (M6 close): the far ground carries the water the `water` option draws, as
##   one more mesh hidden by the same texture, and a skirt under each far chunk's edges, so neither
##   a far tarn nor the slit where a loaded chunk's fine edge meets a far one opens on the sky.
## - Far meshes (M6 close): every generated mesh also has a copy with no body, made once, shown
##   exactly while the mesh is not loaded, so a deck or a cliff face past the load radius is drawn
##   as whole draws it (`far_meshes = false` leaves them out).

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
## Lockstep: a chunk begun on the worker is waited for and added on the next update, so what is
## loaded after each update depends only on the points given, never on the worker's pace. A
## scripted walk (walk.gd) sets it: the bodies enter the physics space in the same order on every
## run, and the walk ends in the same place (M6 slice 2; a walk is sensitive to that order).
var lockstep := false
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
var _far_things: MeshInstance3D  # the far instances (Loader.make_far_instances), or null
## Whether the far instances are made (before the streamer's first chunk; read at construction).
static var far_instances := true
## Whether the far meshes are made (read at construction).
static var far_meshes := true
var _far_mesh_nodes := {}  # mesh index -> its copy with no body, shown while it is not loaded
var _far_holder: Node3D
var _far_mask: Image            # one R8 texel per chunk, at `[cx, cy]`: 255 while it is loaded
var _far_tex: ImageTexture
var _ats: Array[Vector2i] = []  # chunk index -> its `at`
var _task := -1
var _task_n := -1
var _result := {}


func _init(the_loader, the_root: Node3D, radius := LOAD) -> void:
	_setup(the_loader, the_root)
	set_radius(radius)


func _setup(the_loader, the_root: Node3D) -> void:
	## The chunk rectangles, the far ground and which meshes each chunk holds, for one build.
	loader = the_loader
	root = the_root
	_rects.clear()
	_ats.clear()
	_chunk_meshes.clear()
	_side = 0.0
	var chunks: Array = loader.chunk_list()
	for n in chunks.size():
		var r: Rect2 = loader.chunk_rect(n)
		_rects.append(r)
		_side = maxf(_side, maxf(r.size.x, r.size.y))
		_chunk_meshes.append([])
	var size := Vector2i(1, 1)
	for c in chunks:
		var at: Array = c.get("at", [0, 0])
		_ats.append(Vector2i(int(at[0]), int(at[1])))
		size = size.max(_ats[-1] + Vector2i(1, 1))
	_far_tex = null
	_far = loader.make_far_ground()
	if _far != null:
		_far_mask = Image.create_empty(size.x, size.y, false, Image.FORMAT_R8)
		_far_tex = ImageTexture.create_from_image(_far_mask)
		_far.material_override.set_shader_parameter("loaded", _far_tex)
		for w in _far.get_children():   # the far water
			w.material_override.set_shader_parameter("loaded", _far_tex)
		root.add_child(_far)
	_far_things = loader.make_far_instances() if far_instances else null
	if _far_things != null:
		if _far_tex == null:
			_far_mask = Image.create_empty(size.x, size.y, false, Image.FORMAT_R8)
			_far_tex = ImageTexture.create_from_image(_far_mask)
		_far_things.material_override.set_shader_parameter("loaded", _far_tex)
		root.add_child(_far_things)
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
	_holder = null
	if not _meshes.is_empty():
		_holder = Node3D.new()
		_holder.name = "meshes"
		root.add_child(_holder)
	_far_mesh_nodes.clear()
	_far_holder = null
	if far_meshes and not _meshes.is_empty():
		_far_holder = Node3D.new()
		_far_holder.name = "far_meshes"
		root.add_child(_far_holder)
		for i in _meshes.size():
			var m = loader.make_mesh(_meshes[i])
			if m == null:
				continue
			for body in m.get_children():
				m.remove_child(body)
				body.free()
			_far_holder.add_child(m)
			_far_mesh_nodes[i] = m


## The far mask's colours (ours; the sheet's `--far-mask`, compare_views' "why"): the far layers
## drawn unshaded in colours no pack colour takes, so a view drawn again with the mask on differs
## from the plain one exactly where a far layer shows, and the hue says which.
const MASK_INSTANCES := Color(1, 0, 1)
const MASK_GROUND := Color(0, 1, 1)
var _plain := {}   # node -> its own material while the mask is on


func set_far_mask(on: bool) -> void:
	## Draws the far instances in MASK_INSTANCES and the far ground, its skirts and water and the
	## far meshes in MASK_GROUND, unshaded, each still hidden where its chunk is loaded; off puts
	## their own materials back.
	if not on:
		for n in _plain:
			if is_instance_valid(n):
				n.material_override = _plain[n]
		_plain.clear()
		return
	if not _plain.is_empty():
		return
	var nodes: Array = []
	if _far != null:
		nodes.append([_far, MASK_GROUND])
		for w in _far.get_children():
			nodes.append([w, MASK_GROUND])
	if _far_things != null:
		nodes.append([_far_things, MASK_INSTANCES])
	for i in _far_mesh_nodes:
		nodes.append([_far_mesh_nodes[i], MASK_GROUND])
	for e in nodes:
		var n: GeometryInstance3D = e[0]
		_plain[n] = n.material_override
		var shader := Shader.new()
		shader.code = "shader_type spatial;\nrender_mode unshaded, cull_disabled;\nuniform sampler2D loaded : filter_nearest, repeat_disable;\nuniform bool masked;\nuniform vec3 colour;\nvoid fragment() {\n\tif (masked && texelFetch(loaded, ivec2(floor(UV)), 0).r > 0.5) {\n\t\tdiscard;\n\t}\n\tALBEDO = colour;\n}\n"
		var mat := ShaderMaterial.new()
		mat.shader = shader
		mat.set_shader_parameter("colour", Vector3(e[1].r, e[1].g, e[1].b))
		mat.set_shader_parameter("masked", n == _far or n == _far_things or n.get_parent() == _far)
		mat.set_shader_parameter("loaded", _far_tex)
		n.material_override = mat


func _far_mesh_shown(i: int, shown: bool) -> void:
	if _far_mesh_nodes.has(i):
		_far_mesh_nodes[i].visible = shown


func far_mesh_count() -> int:
	return _far_mesh_nodes.size()


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
	return _far_tex != null and _far_mask.get_pixelv(_ats[n]).r < 0.5


func far_count() -> int:
	## Chunks the far ground covers: every chunk, or none without a surface field.
	return _ats.size() if _far != null else 0


func _far_mark(n: int, is_loaded: bool) -> void:
	if _far_tex != null:
		_far_mask.set_pixelv(_ats[n], Color(1.0 if is_loaded else 0.0, 0, 0))
		_far_tex.update(_far_mask)


func meshes_of(n: int) -> Array:
	return _chunk_meshes[n]


func update(p: Vector2) -> void:
	var t0 := Time.get_ticks_usec()
	if _task >= 0 and (lockstep or WorkerThreadPool.is_task_completed(_task)):
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
		_far_mesh_shown(i, not _mesh_nodes.has(i))


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
			_far_mesh_shown(i, true)


func reload(new_loader, new_root: Node3D) -> Dictionary:
	## After a rebuild (build.md 5.2): streams `new_root` (new_loader.prepare of the new build)
	## instead, keeping each loaded chunk whose `digest` is the same in both manifests, moved to the
	## new root as it is, and making again the loaded chunks whose digest changed (or is missing in
	## either, or whose chunk is gone, or when the frame changed), so the same chunks are loaded
	## after as before. A loaded generated mesh is kept when the new manifest lists it by the same
	## name with the same entry and file digest. The caller frees the old root (its features,
	## paths, bookmarks and far ground are the old build's). Returns {"kept", "made", "dropped"}:
	## chunk `at`s, and "meshes_kept", mesh names, each sorted.
	end()
	var old_list: Array = loader.chunk_list()
	var new_list: Array = new_loader.chunk_list()
	var same_frame: bool = JSON.stringify(loader.frame()) == JSON.stringify(new_loader.frame())
	var new_index := {}
	for m in new_list.size():
		var at: Array = new_list[m].get("at", [0, 0])
		new_index[Vector2i(int(at[0]), int(at[1]))] = m
	var keep := {}   # new index -> node
	var remake: Array[int] = []
	var report := {"kept": [], "made": [], "dropped": [], "meshes_kept": []}
	for n in loaded.keys():
		var node: Node3D = loaded[n]
		var at := _ats[n]
		var m: int = new_index.get(at, -1)
		var d = old_list[n].get("digest")
		var same: bool = same_frame and m >= 0 and d is String and d != "" and new_list[m].get("digest") == d
		root.remove_child(node)
		if same:
			keep[m] = node
			report.kept.append([at.x, at.y])
		else:
			node.free()
			if m >= 0:
				remake.append(m)
				report.made.append([at.x, at.y])
			else:
				report.dropped.append([at.x, at.y])
	# Generated meshes: keep a node whose name, entry and file digest are unchanged.
	var mesh_keep := {}   # name -> node
	var new_meshes: Dictionary = {}
	for e in new_loader.mesh_list():
		if e is Dictionary:
			new_meshes[e.get("name")] = e
	for i in _mesh_nodes:
		var e = _meshes[i]
		var node: Node = _mesh_nodes[i]
		_holder.remove_child(node)
		var ne = new_meshes.get(e.get("name")) if e is Dictionary else null
		if same_frame and ne != null and JSON.stringify(ne) == JSON.stringify(e) \
				and loader.file_digest(String(e.get("file", ""))) == new_loader.file_digest(String(ne.get("file", ""))) \
				and loader.file_digest(String(e.get("file", ""))) != "":
			mesh_keep[e.name] = node
			report.meshes_kept.append(String(e.name))
		else:
			node.free()
	for far in [_far, _far_things, _far_holder]:
		if far != null and far.get_parent() == root:
			root.remove_child(far)
			far.free()
	loaded.clear()
	_mesh_nodes.clear()
	_mesh_refs.clear()
	_setup(new_loader, new_root)
	set_radius(load_radius)
	for i in _meshes.size():
		if _meshes[i] is Dictionary and mesh_keep.has(_meshes[i].get("name")):
			_mesh_nodes[i] = mesh_keep[_meshes[i].name]
			_holder.add_child(_mesh_nodes[i])
			mesh_keep.erase(_meshes[i].name)
	for node in mesh_keep.values():
		node.free()   # its chunks are gone
	var ms := keep.keys()
	ms.sort()
	for m in ms:
		_attach(m, {"chunk": keep[m], "meshes": {}})
	remake.sort()
	for m in remake:
		_attach(m, _made(m, _wanted(m)))
	loads -= keep.size()   # a kept chunk is not a load
	for i in _mesh_nodes.keys():
		if _mesh_refs.get(i, 0) <= 0:   # kept, but no loaded chunk holds it now
			_holder.remove_child(_mesh_nodes[i])
			_mesh_nodes[i].free()
			_mesh_nodes.erase(i)
			_far_mesh_shown(i, true)
	for k in ["kept", "made", "dropped", "meshes_kept"]:
		report[k].sort()
	return report

