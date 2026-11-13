extends Node
## The sheet tool of spec/build.md 15.1: renders every bookmark of one build, or of two builds
## before and after, at eye height, and lays them out on one sheet. The loader's code, run by Godot:
##
##   godot --path engines/godot --rendering-method gl_compatibility res://sheet.tscn -- \
##         --before DIR [--after DIR] --out DIR [--pack DIR] [--option NAME=VALUE ...]
##
## engines/godot/sheet.sh runs it with a watchdog. It needs a display: Godot's --headless display
## draws nothing. Writes <out>/before/*.png, <out>/after/*.png (1280 by 720), <out>/sheet.png,
## <out>/sheet.json and <out>/diagnostics.json. <out> is written whole: into <out>.partial, then
## swapped in. Exit 0 when every bookmark was rendered, 1 with any error, 2 for a bad argument.
## A watchdog quits with 3 if the run takes longer than WATCHDOG_S, so a stuck renderer never
## leaves an unattended run waiting.

const Loader := preload("res://addons/via_loader/loader.gd")
const Diagnostics := preload("res://addons/via_loader/diagnostics.gd")
const CJson := preload("res://addons/via_loader/cjson.gd")
const Compose := preload("res://compose.gd")
const Rig := preload("res://rig.gd")

const SHOT := Vector2i(1280, 720)
const SETTLE_FRAMES := 4
const WATCHDOG_S := 300.0
const PARKED := Vector2i(160, 90)
const USAGE := "usage: godot --path engines/godot --rendering-method gl_compatibility res://sheet.tscn -- --before DIR [--after DIR] --out DIR [--pack DIR] [--option NAME=VALUE ...]"

var before := ""
var after := ""
var out := ""
var pack := ""
var options := {}
var diags := Diagnostics.new()
var _done := false


func _ready() -> void:
	get_tree().create_timer(WATCHDOG_S, true, false, true).timeout.connect(_watchdog)
	var why := _read_arguments()
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
		printerr("sheet: watchdog: no result after %d s; quitting" % int(WATCHDOG_S))
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
		var takes := a in ["--before", "--after", "--out", "--pack", "--option"]
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
	var loader := Loader.new()
	loader.pack_dir = pack
	loader.diags.build_label = label
	var root := loader.make(dir, options)
	diags.records.append_array(loader.diags.records)
	var res := {"id": null, "world": null, "shots": {}, "far": null, "option_error": loader.option_error}
	if root == null:
		return res
	res.id = root.get_meta("id")
	res.world = root.get_meta("world")
	vp.add_child(root)
	DirAccess.make_dir_recursive_absolute(folder)
	for cam in root.get_node("bookmarks").get_children():
		var c := cam as Camera3D
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
	root.queue_free()
	await RenderingServer.frame_post_draw
	return res


func _run() -> void:
	var partial := out + ".partial"
	_remove(partial)
	DirAccess.make_dir_recursive_absolute(partial)
	var vp := SubViewport.new()
	vp.size = SHOT
	vp.msaa_3d = Viewport.MSAA_4X
	vp.own_world_3d = true
	vp.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(vp)
	Rig.rig(vp)
	var two := after != ""
	var b: Dictionary = await _render(vp, before, "before", partial + "/before")
	var a: Dictionary = {"shots": {}, "id": null, "world": null, "far": null, "option_error": ""}
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
	var doc := {
		"after": {"id": a.id, "world": a.world} if two else null,
		"before": {"id": b.id, "world": b.world},
		"bookmarks": marks,
		"camera": {"far": b.far if b.far != null else a.far, "fov": 75.0, "near": 0.05},
		"godot": godot,
		"light": {"bearing": Rig.LIGHT_BEARING, "elevation": Rig.LIGHT_ELEVATION},
		"renderer": RenderingServer.get_current_rendering_method(),
		"size": [SHOT.x, SHOT.y],
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
