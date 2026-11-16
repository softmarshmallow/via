extends Node
## The sheet tool of spec/build.md 15.1: renders every bookmark of one build, or of two builds
## before and after, at eye height, and lays them out on one sheet. The loader's code, run by Godot:
##
##   godot --path engines/godot --rendering-method gl_compatibility res://sheet.tscn -- \
##         --before DIR [--after DIR] --out DIR [--pack DIR] [--option NAME=VALUE ...]
##         [--stream RADIUS] [--frames SECONDS]
##
## --stream loads chunks round each eye and walker within RADIUS metres (stream.gd) instead of the
## whole build; --frames tours the bookmarks' eyes at TOUR_SPEED for SECONDS per build after the
## walks and records the frame times (sheet.json `frames`).
##
## engines/godot/sheet.sh runs it with a watchdog. It needs a display: Godot's --headless display
## draws nothing. Writes <out>/before/*.png, <out>/after/*.png (1280 by 720), <out>/sheet.png,
## <out>/sheet.json and <out>/diagnostics.json. <out> is written whole: into <out>.partial, then
## swapped in. Exit 0 when every bookmark was rendered, 1 with any error, 2 for a bad argument.
## A watchdog quits with 3 if the run takes longer than WATCHDOG_S (plus twice --frames), so a stuck renderer never
## leaves an unattended run waiting.

const Loader := preload("res://addons/via_loader/loader.gd")
const Diagnostics := preload("res://addons/via_loader/diagnostics.gd")
const CJson := preload("res://addons/via_loader/cjson.gd")
const Compose := preload("res://compose.gd")
const Rig := preload("res://rig.gd")
const Walk := preload("res://walk.gd")
const Marks := preload("res://marks.gd")
const Stream := preload("res://addons/via_loader/stream.gd")

const SHOT := Vector2i(1280, 720)
const SETTLE_FRAMES := 4
const WATCHDOG_S := 300.0
const PARKED := Vector2i(160, 90)
const USAGE := "usage: godot --path engines/godot --rendering-method gl_compatibility res://sheet.tscn -- --before DIR [--after DIR] --out DIR [--pack DIR] [--option NAME=VALUE ...] [--stream RADIUS] [--frames SECONDS]"
## The frame tour's pace, metres a second (ours): a run, five times the walker's, so a tour of a
## minute crosses a city's chunks.
const TOUR_SPEED := 7.0

var before := ""
var after := ""
var out := ""
var pack := ""
var options := {}
var diags := Diagnostics.new()
var edits := {"against": null, "pins": [], "cleared": []}
var stream_radius := 0.0
var frames_s := 0.0
var _done := false
var _watchdog_s := WATCHDOG_S


func _ready() -> void:
	var why := _read_arguments()
	# a frame tour runs its seconds on each build, past the watchdog's own time
	_watchdog_s = WATCHDOG_S + 2.0 * maxf(frames_s, 0.0)
	get_tree().create_timer(_watchdog_s, true, false, true).timeout.connect(_watchdog)
	if why == "":
		why = _check_out()
	if why != "":
		printerr("sheet: " + why)
		printerr(USAGE)
		_quit(2)
		return
	_park_window()
	await _run()


func _watchdog() -> void:
	if not _done:
		printerr("sheet: watchdog: no result after %d s; quitting" % int(_watchdog_s))
		_quit(3)


func _quit(code: int) -> void:
	_done = true
	get_tree().quit(code)


func _abs(path: String) -> String:
	## A relative path is taken from the shell's working directory, not Godot's (the project).
	if path == "" or path.is_absolute_path():
		return path.simplify_path()
	var cwd := OS.get_environment("PWD")
	return (cwd + "/" + path).simplify_path() if cwd != "" else path


func _read_arguments() -> String:
	var args := OS.get_cmdline_user_args()
	var n := 0
	while n < args.size():
		var a := args[n]
		var takes := a in ["--before", "--after", "--out", "--pack", "--option", "--stream", "--frames"]
		if not takes:
			return "unknown argument `%s`" % a
		if n + 1 >= args.size():
			return "`%s` takes a value" % a
		var v := args[n + 1]
		match a:
			"--before": before = _abs(v)
			"--after": after = _abs(v)
			"--out": out = _abs(v)
			"--pack": pack = _abs(v)
			"--stream", "--frames":
				if not v.is_valid_float() or float(v) <= 0.0:
					return "%s takes a number above 0, not `%s`" % [a, v]
				if a == "--stream":
					stream_radius = float(v)
				else:
					frames_s = float(v)
			"--option":
				var eq := v.find("=")
				if eq <= 0:
					return "--option takes NAME=VALUE, not `%s`" % v
				var name := v.substr(0, eq)
				if name == "diagnostics":
					return "--option diagnostics is not taken: the tool writes <out>/diagnostics.json"
				var err := Loader.parse_option(options, name, v.substr(eq + 1))
				if err != "":
					return "--option " + err
		n += 2
	if before == "" or out == "":
		return "--before and --out are required"
	return ""


static func _ours(dir: String) -> bool:
	## A folder this tool wrote (it holds sheet.json), or an empty one.
	return FileAccess.file_exists(dir + "/sheet.json") or DirAccess.get_files_at(dir).is_empty() and DirAccess.get_directories_at(dir).is_empty()


func _check_out() -> String:
	for d in [out, out + ".partial", out + ".old"]:
		if FileAccess.file_exists(d):
			return "%s is a file; --out names a folder" % d
		if DirAccess.dir_exists_absolute(d) and not _ours(d):
			return "%s is a folder this tool did not write (no sheet.json); it is not replaced" % d
	return ""


func _park_window() -> void:
	## The picture is drawn off screen; the window is a small sliver kept out of the way.
	var win := get_window()
	win.unfocusable = true
	win.borderless = true
	win.size = PARKED
	var screen := DisplayServer.window_get_current_screen()
	win.position = DisplayServer.screen_get_position(screen) + DisplayServer.screen_get_size(screen) - PARKED - Vector2i(8, 8)


static func _remove(dir: String) -> void:
	if not DirAccess.dir_exists_absolute(dir):
		return
	for d in DirAccess.get_directories_at(dir):
		_remove(dir + "/" + d)
	for f in DirAccess.get_files_at(dir):
		DirAccess.remove_absolute(dir + "/" + f)
	DirAccess.remove_absolute(dir)


func _render(vp: SubViewport, dir: String, label: String, folder: String) -> Dictionary:
	## Loads one build into the viewport and renders each bookmark camera. Returns
	## { "root_meta": {id, world}, "shots": { name: {"file", "eye", "toward", "image"} }, "far" }.
	# Each build in a fresh physics space: a walk over a build loaded into the space the other
	# build's bodies had been in and left ended elsewhere (border `bank_corner` 40.7 m off, M6
	# slice 2): the space's past is not the build's.
	var was := vp.world_3d   # held through the switch: freed first, its scenario is gone when left
	vp.world_3d = World3D.new()
	was = null
	var loader := Loader.new()
	loader.pack_dir = pack
	loader.diags.build_label = label
	var root := loader.prepare(dir, options) if stream_radius > 0.0 else loader.make(dir, options)
	diags.records.append_array(loader.diags.records)
	var res := {"id": null, "world": null, "shots": {}, "walks": {}, "far": null, "option_error": loader.option_error, "frames": null}
	if root == null:
		return res
	res.id = root.get_meta("id")
	res.world = root.get_meta("world")
	if Marks.count(edits) > 0:
		root.add_child(Marks.node(edits, loader))
	vp.add_child(root)
	var stream = Stream.new(loader, root, stream_radius) if stream_radius > 0.0 else null
	DirAccess.make_dir_recursive_absolute(folder)
	for cam in root.get_node("bookmarks").get_children():
		var c := cam as Camera3D
		_near(stream, Vector2(c.get_meta("eye")[0], c.get_meta("eye")[1]))
		c.make_current()
		res.far = c.get_meta("far")
		for f in SETTLE_FRAMES:
			await RenderingServer.frame_post_draw
		var img := vp.get_texture().get_image()
		img.convert(Image.FORMAT_RGB8)
		var file := "%s/%s.png" % [label, c.name]
		var err := img.save_png(out + ".partial/" + file)
		if err != OK:
			printerr("sheet: cannot write %s (%s)" % [file, error_string(err)])
			res["write_failed"] = true
			continue
		res.shots[String(c.get_meta("feature"))] = {"file": file, "eye": c.get_meta("eye"), "toward": c.get_meta("toward"), "image": img}
	await _walks(vp, root, loader, label, folder, res, stream)
	if frames_s > 0.0:
		res.frames = await _frames(root, loader, stream)
	if stream != null:
		stream.end()
	root.queue_free()
	await RenderingServer.frame_post_draw
	return res


static func _near(stream, p: Vector2) -> void:
	## Streaming: the chunks within the load radius of p made now, the far ones freed, for a still.
	if stream != null:
		stream.update(p)
		stream.fill(p, stream.load_radius)


func _frames(root: Node3D, loader, stream) -> Dictionary:
	## The frame tour (ours; challenge 4): a camera at the walker's eye over the ground goes from
	## each bookmark's eye to the next in name order and back to the first, at TOUR_SPEED, facing
	## the way it goes, for frames_s seconds of the machine's time, rendering the 1280 by 720 view
	## every frame with no frame cap. Records each frame's time (one drawn frame to the next), and
	## with streaming the streamer's own time on the frame and its counts, and the renderer's own
	## measure of the view's draw (view_ms: cpu and gpu, 0 where the renderer gives none). The first second is not
	## counted (shaders compile and the first chunks come in).
	var route: Array[Vector2] = []
	var cams := root.get_node("bookmarks").get_children()
	cams.sort_custom(func(a, b): return String(a.get_meta("feature")) < String(b.get_meta("feature")))
	for c in cams:
		var e: Array = c.get_meta("eye")
		route.append(Vector2(e[0], e[1]))
	if route.size() < 2:
		return {"error": "fewer than two bookmarks: no route"}
	route.append(route[0])
	var cam := Camera3D.new()
	cam.far = cams[0].far
	root.add_child(cam)
	cam.make_current()
	var vrid := root.get_viewport().get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(vrid, true)
	var gpu: Array[float] = []
	var cpu: Array[float] = []
	var vsync := DisplayServer.window_get_vsync_mode()
	var cap := Engine.max_fps
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	if stream != null:
		stream.update(route[0])
		stream.fill(route[0], stream.load_radius)
		stream.loads = 0
		stream.unloads = 0
		stream.stalls = 0
		stream.work_max_usec = 0
	var p := route[0]
	var next := 1
	var times: Array[float] = []
	var work: Array[float] = []
	var most := 0
	var start := Time.get_ticks_usec()
	var last := start
	var walked := 0.0
	while true:
		var now := Time.get_ticks_usec()
		var dt := (now - last) / 1e6
		last = now
		if now - start > 1e6:
			times.append(dt * 1000.0)
			gpu.append(RenderingServer.viewport_get_measured_render_time_gpu(vrid))
			cpu.append(RenderingServer.viewport_get_measured_render_time_cpu(vrid))
		if now - start > (frames_s + 1.0) * 1e6:
			break
		var left := TOUR_SPEED * minf(dt, 0.1)
		while left > 0.0:
			var d := p.distance_to(route[next])
			if d <= left:
				left -= d
				walked += d
				p = route[next]
				next = next % (route.size() - 1) + 1
			else:
				p += (route[next] - p) / d * left
				walked += left
				left = 0.0
		var ahead := route[next] - p
		var dir := ahead.normalized() if ahead.length() > 0.01 else Vector2.UP
		var z: float = loader.ground_at(p.x, p.y) if loader.has_ground() else 0.0
		z = maxf(z, z + loader.depth_at(p.x, p.y)) + Walk.EYE
		cam.transform = Transform3D(Basis.looking_at(Vector3(dir.x, 0, -dir.y), Vector3.UP), Loader.g(p.x, p.y, z))
		if stream != null:
			stream.update(p)
			if now - start > 1e6:
				work.append(stream.work_usec / 1000.0)
			most = maxi(most, stream.loaded.size())
		await RenderingServer.frame_post_draw
	DisplayServer.window_set_vsync_mode(vsync)
	Engine.max_fps = cap
	cam.queue_free()
	var r := {"seconds": frames_s, "speed": TOUR_SPEED, "toured_m": walked, "frames": times.size(),
		"fps": _fps(times), "frame_ms": _spread(times), "stream": null,
		"view_ms": {"cpu": _spread(cpu), "gpu": _spread(gpu)}}
	if stream != null:
		r.stream = {"load_radius": stream.load_radius, "unload_radius": stream.unload_radius,
			"guard": stream.guard, "loads": stream.loads, "unloads": stream.unloads, "stalls": stream.stalls,
			"most_loaded": most, "chunks": stream.chunk_count(), "work_ms": _spread(work)}
	return r


static func _spread(v: Array[float]) -> Dictionary:
	if v.is_empty():
		return {"median": null, "p99": null, "max": null}
	var s := v.duplicate()
	s.sort()
	return {"median": s[s.size() / 2], "p99": s[mini(s.size() - 1, int(s.size() * 0.99))], "max": s[-1]}


static func _fps(times: Array[float]) -> Dictionary:
	## Frames a second: median (1000 / the median frame time), the slowest 1% (1000 / the 99th
	## percentile frame time) and the minimum (1000 / the longest frame).
	var s := _spread(times)
	if s.median == null:
		return {"median": null, "low_1pct": null, "min": null}
	return {"median": 1000.0 / s.median, "low_1pct": 1000.0 / s.p99, "min": 1000.0 / s.max}


func _walks(vp: SubViewport, root: Node3D, loader, label: String, folder: String, res: Dictionary, stream = null) -> void:
	## Walks from each bookmark toward its target (walk.gd), then renders the walk from above with
	## its trail and from the walker's eye where it stopped: <label>/walk/<name>_plan.png and
	## <label>/walk/<name>_eye.png.
	var unbodied: Array = loader.solid_without_body()
	DirAccess.make_dir_recursive_absolute(folder + "/walk")
	for cam in root.get_node("bookmarks").get_children():
		var name := String(cam.get_meta("feature"))
		var e: Array = cam.get_meta("eye")
		var t: Dictionary = cam.get_meta("toward")
		_near(stream, Vector2(e[0], e[1]))
		var r: Dictionary = await Walk.walk(root, loader, Vector3(e[0], e[1], e[2]), Vector3(t.at[0], t.at[1], t.at[2]), String(t.name), unbodied,
			cam.get_meta("along", []), stream)
		_near(stream, Vector2(r.stop[0], r.stop[1]))
		var trail := Walk.trail_node(r)
		root.add_child(trail)
		var shots := {}
		for view in ["plan", "eye"]:
			var vcam: Camera3D = Walk.overhead(r, float(SHOT.x) / SHOT.y) if view == "plan" else Walk.eye(r, root.get_meta("frame").extent, loader)
			root.add_child(vcam)
			vcam.make_current()
			for f in SETTLE_FRAMES:
				await RenderingServer.frame_post_draw
			var img := vp.get_texture().get_image()
			img.convert(Image.FORMAT_RGB8)
			var file := "%s/walk/%s_%s.png" % [label, cam.name, view]
			if img.save_png(out + ".partial/" + file) != OK:
				printerr("sheet: cannot write %s" % file)
				res["write_failed"] = true
			shots[view] = {"file": file, "image": img}
			vcam.queue_free()
		root.remove_child(trail)
		trail.free()
		r.erase("trail")
		r.erase("facing")
		r["plan"] = shots.plan.file
		r["eye"] = shots.eye.file
		res.walks[name] = {"report": r, "plan": shots.plan.image, "eye": shots.eye.image}


func _run() -> void:
	var partial := out + ".partial"
	_remove(partial)
	DirAccess.make_dir_recursive_absolute(partial)
	var vp := SubViewport.new()
	vp.size = SHOT
	vp.msaa_3d = Viewport.MSAA_4X
	vp.world_3d = World3D.new()   # and a fresh one for each build (_render)
	vp.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(vp)
	Rig.rig(vp)
	var two := after != ""
	var edited := Marks.read(after if two else before)
	if edited.against != null:
		edits = edited
	var b: Dictionary = await _render(vp, before, "before", partial + "/before")
	var a: Dictionary = {"shots": {}, "walks": {}, "id": null, "world": null, "far": null, "option_error": "", "frames": null}
	if two:
		a = await _render(vp, after, "after", partial + "/after")
	vp.queue_free()
	for r in [b, a]:
		if r.option_error != "":
			printerr("sheet: --option " + r.option_error)
			_remove(partial)
			_quit(2)
			return
	var names := {}
	for n in b.shots:
		names[n] = true
	for n in a.shots:
		names[n] = true
	var order := names.keys()
	order.sort()
	var rows: Array = []
	var marks: Array = []
	for n in order:
		# Each cell is seen from its own build's eye toward its own build's target: both are
		# recorded, and labelled when they differ.
		var eb = b.shots.get(n, {}).get("eye")
		var ea = a.shots.get(n, {}).get("eye")
		var tb = b.shots.get(n, {}).get("toward")
		var ta = a.shots.get(n, {}).get("toward")
		var eyes := _eye_text(eb if eb != null else ea)
		if eb != null and ea != null and eb != ea:
			eyes = "%s -> %s" % [_eye_text(eb), _eye_text(ea)]
		var towards := String((tb if tb != null else ta).get("name", ""))
		if tb != null and ta != null and tb.get("name") != ta.get("name"):
			towards = "%s -> %s" % [tb.get("name", ""), ta.get("name", "")]
		rows.append({
			"label": "%s    eye %s    toward %s" % [n, eyes, towards],
			"before": b.shots.get(n, {}).get("image"), "after": a.shots.get(n, {}).get("image"),
		})
		marks.append({"after": a.shots.get(n, {}).get("file") if two else null,
			"before": b.shots.get(n, {}).get("file"), "eye": {"after": ea, "before": eb}, "name": n,
			"target": {"after": ta, "before": tb}})
	var godot := "%d.%d.%d.%s" % [Engine.get_version_info().major, Engine.get_version_info().minor,
		Engine.get_version_info().patch, Engine.get_version_info().status]
	var world = b.world if b.world != null else a.world
	var header := "%s    before %s" % [world if world != null else "(no build)", _short(b.id)]
	if two:
		header += "    after %s" % _short(a.id)
	header += "    Godot %s" % godot
	var sheet: Image = await Compose.compose(self, header, rows, two)
	var failed := sheet.save_png(partial + "/sheet.png") != OK or b.has("write_failed") or a.has("write_failed")
	# the walks: one row from above and one at the walker's eye per bookmark, before and after
	var wrows: Array = []
	var walks: Array = []
	for n in order:
		var wb = b.walks.get(n)
		var wa = a.walks.get(n)
		var rb = wb.report if wb != null else null
		var ra = wa.report if wa != null else null
		var line := "walk %s toward %s    before: %s" % [n, (rb if rb != null else ra).toward, Walk.summary(rb)]
		if two:
			line += "    after: %s" % Walk.summary(ra)
		wrows.append({"label": line + "    (from above)",
			"before": wb.plan if wb != null else null, "after": wa.plan if wa != null else null})
		wrows.append({"label": "walk %s    at the walker's eye, %.0f m short of where it stopped, facing on (back, at the world's edge)" % [n, Walk.BACK],
			"before": wb.eye if wb != null else null, "after": wa.eye if wa != null else null})
		walks.append({"after": ra if two else null, "before": rb, "name": n})
	var wheader := "%s    walks    pins %d, cleared %d" % [world if world != null else "(no build)",
		edits.pins.size(), edits.cleared.size()]
	var wsheet: Image = await Compose.compose(self, wheader, wrows, two)
	failed = failed or wsheet.save_png(partial + "/walks.png") != OK
	var doc := {
		"after": {"id": a.id, "world": a.world} if two else null,
		"before": {"id": b.id, "world": b.world},
		"bookmarks": marks,
		"camera": {"far": b.far if b.far != null else a.far, "fov": 75.0, "near": 0.05},
		"edits": edits,
		"frames": {"after": a.frames, "before": b.frames} if frames_s > 0.0 else null,
		"godot": godot,
		"light": {"bearing": Rig.LIGHT_BEARING, "elevation": Rig.LIGHT_ELEVATION},
		"renderer": RenderingServer.get_current_rendering_method(),
		"size": [SHOT.x, SHOT.y],
		"stream": stream_radius if stream_radius > 0.0 else null,
		"walk": {"arrive": Walk.ARRIVE, "eye": Walk.EYE, "fall": Walk.FALL, "height": Walk.HEIGHT,
			"max_slope": Walk.MAX_SLOPE, "radius": Walk.RADIUS, "speed": Walk.SPEED, "step": Walk.STEP, "wade": Walk.WADE,
			"above_water": Walk.ABOVE_WATER},
		"walks": walks,
	}
	var f := FileAccess.open(partial + "/sheet.json", FileAccess.WRITE)
	f.store_string(CJson.text(doc))
	f.close()
	diags.write(partial + "/diagnostics.json")
	diags.print_all()
	# swap: the old folder aside, the new one in, the old one deleted
	_remove(out + ".old")
	if DirAccess.dir_exists_absolute(out):
		DirAccess.rename_absolute(out, out + ".old")
	var err := DirAccess.rename_absolute(partial, out)
	_remove(out + ".old")
	if err != OK:
		printerr("sheet: cannot move %s into place (%s)" % [partial, error_string(err)])
		failed = true
	print("sheet: %d bookmark(s), %s" % [order.size(), out + "/sheet.png"])
	_quit(1 if failed or diags.status() != 0 else 0)


static func _eye_text(e: Array) -> String:
	return "(%.2f, %.2f, %.2f)" % [e[0], e[1], e[2]]


static func _short(id) -> String:
	return String(id).substr(0, 12) if id is String else "(none)"
