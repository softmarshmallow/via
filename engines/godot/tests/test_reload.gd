extends SceneTree
## Reload by chunk digest after a rebuild (spec/build.md 5.2 and 15.1, "Streaming", M6 slice 2):
##   godot --headless --path engines/godot --script res://tests/test_reload.gd -- \
##         --before DIR --after DIR [--at X Y] [--radius M]
## Headless: data and physics only. Two builds of one world, before and after a change. The before
## build is streamed and then handed the after build (stream.gd `reload`), twice: with every chunk
## loaded, and with the chunks within the radius of a point (default the extent's centre).
##   - the chunks kept are exactly the loaded ones whose `digest` is the same in both manifests,
##     each the same node as before; the others loaded are made again, from the after build;
##   - the loaded chunks are the same `at`s after as before;
##   - with every chunk loaded, the streamed root then holds what the after build made whole holds
##     (instances, bodies, generated meshes, chunks), each once;
##   - with --mixed, with every chunk loaded at least one chunk is kept and one made again (the
##     loader suite's fixtures: the ford's move changes three of four chunks' digests).
## Prints one line per check and exits 0 when all pass, 1 otherwise.

const Loader := preload("res://addons/via_loader/loader.gd")
const Stream := preload("res://addons/via_loader/stream.gd")
const StreamTest := preload("res://tests/test_stream.gd")

var failed := 0
var mixed := false


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


static func digests(dir: String) -> Dictionary:
	var m = JSON.parse_string(FileAccess.get_file_as_string(dir + "/manifest.json"))
	var out := {}
	for c in m.chunks:
		out[Vector2i(int(c.at[0]), int(c.at[1]))] = c.get("digest", "")
	return out


func once(before: String, after: String, at: Vector2, radius: float, what: String) -> void:
	var la := Loader.new()
	var ra := la.prepare(before, {})
	get_root().add_child(ra)
	var stream := Stream.new(la, ra, radius)
	stream.threaded = false
	stream.fill(at, radius)
	var ids := {}   # at -> node instance id
	for n in stream.loaded:
		ids[stream._ats[n]] = stream.loaded[n].get_instance_id()
	var da := digests(before)
	var db := digests(after)
	var want_kept: Array = []
	var want_made: Array = []
	for a in ids:
		if da[a] != "" and db.get(a, "") == da[a]:
			want_kept.append([a.x, a.y])
		elif db.has(a):
			want_made.append([a.x, a.y])
	want_kept.sort()
	want_made.sort()
	var lb := Loader.new()
	var rb := lb.prepare(after, {})
	get_root().add_child(rb)
	# the loaded meshes whose entry and file digest are the same in both manifests
	var ma = JSON.parse_string(FileAccess.get_file_as_string(before + "/manifest.json"))
	var mb = JSON.parse_string(FileAccess.get_file_as_string(after + "/manifest.json"))
	var entries := {}
	for e in mb.get("meshes", []):
		entries[e.name] = e
	var want_meshes: Array = []
	var loaded_meshes := 0
	var holder := ra.get_node_or_null("meshes")
	for e in ma.get("meshes", []):
		if holder == null or holder.get_node_or_null(NodePath(Loader.node_name(e.name))) == null:
			continue
		loaded_meshes += 1
		var ne = entries.get(e.name)
		if ne != null and JSON.stringify(ne) == JSON.stringify(e) and ma.files[e.file].digest == mb.files[ne.file].digest:
			want_meshes.append(String(e.name))
	want_meshes.sort()
	var t0 := Time.get_ticks_usec()
	var r := stream.reload(lb, rb)
	var ms := (Time.get_ticks_usec() - t0) / 1000.0
	ra.free()
	check("%s: kept exactly the loaded chunks whose digest is unchanged" % what, r.kept == want_kept,
		"%d kept, %d expected, of %d loaded" % [r.kept.size(), want_kept.size(), ids.size()])
	check("%s: made again exactly the loaded chunks whose digest changed" % what, r.made == want_made and r.dropped.is_empty(),
		"%d made, %d expected, %d dropped; reload %.1f ms" % [r.made.size(), want_made.size(), r.dropped.size(), ms])
	check("%s: kept exactly the loaded meshes whose entry and file digest are unchanged" % what, r.meshes_kept == want_meshes,
		"%d kept, %d expected, of %d loaded" % [r.meshes_kept.size(), want_meshes.size(), loaded_meshes])
	var same_nodes := 0
	var fresh := 0
	var now := {}
	for n in stream.loaded:
		now[stream._ats[n]] = stream.loaded[n]
	for a in ids:
		if not now.has(a):
			continue
		if now[a].get_instance_id() == ids[a]:
			same_nodes += 1
		else:
			fresh += 1
	check("%s: a kept chunk is the same node, a changed one a new node, the same chunks loaded" % what,
		same_nodes == want_kept.size() and fresh == want_made.size() and now.size() == ids.size(),
		"%d same, %d new, %d loaded after against %d before" % [same_nodes, fresh, now.size(), ids.size()])
	if mixed and radius == INF:
		check("%s: at least one chunk kept and one made again" % what, not r.kept.is_empty() and not r.made.is_empty(),
			"%d kept, %d made" % [r.kept.size(), r.made.size()])
	check("%s: every kept or made chunk is under the after build's root" % what,
		now.values().all(func(c): return c.get_parent() == rb))
	if radius == INF:
		var whole := Loader.new().make(after, {})
		var a := StreamTest.content(whole)
		var b := StreamTest.content(rb)
		for key in ["instances", "bodies", "meshes", "chunks"]:
			check("%s: %s as the after build's whole, each once" % [what, key], a[key] == b[key] and StreamTest.dupes(b[key]) == 0,
				"%d against %d, %d twice" % [b[key].size(), a[key].size(), StreamTest.dupes(b[key])])
		whole.free()
	stream.end()
	rb.free()


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var before := ""
	var after := ""
	var at := Vector2(INF, INF)
	var radius := Stream.LOAD
	var k := 0
	while k < args.size():
		match args[k]:
			"--before": before = args[k + 1]
			"--after": after = args[k + 1]
			"--radius": radius = float(args[k + 1])
			"--mixed":
				mixed = true
				k -= 1
			"--at":
				at = Vector2(float(args[k + 1]), float(args[k + 2]))
				k += 1
		k += 2
	if before == "" or after == "":
		printerr("test_reload: --before DIR and --after DIR are required")
		quit(2)
		return
	if at.x == INF:
		var m = JSON.parse_string(FileAccess.get_file_as_string(before + "/manifest.json"))
		var e: Array = m.frame.extent
		at = Vector2((float(e[0]) + float(e[2])) / 2.0, (float(e[1]) + float(e[3])) / 2.0)
	once(before, after, Vector2.ZERO, INF, "every chunk loaded")
	once(before, after, at, radius, "within %.0f m of (%.0f, %.0f)" % [radius, at.x, at.y])
	print("%s: %d check(s) failed" % ["PASS" if failed == 0 else "FAIL", failed])
	quit(0 if failed == 0 else 1)
