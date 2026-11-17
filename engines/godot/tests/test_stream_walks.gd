extends SceneTree
## Streamed walks against whole walks (spec/build.md 15.1, "Streaming", M6 slice 2):
##   godot --headless --path engines/godot --script res://tests/test_stream_walks.gd -- \
##         --build DIR [--radius M] [--only NAME] [--log FILE] [--water SELECTOR]
## Headless: data and physics only. Walks from every bookmark of the build toward its target with
## the sheet's walker (walk.gd), in name order, each load in a fresh physics space (its own
## World3D):
##   whole      Loader.make, every chunk at once;
##   again      the same, a second time in the process;
##   all        Loader.prepare and stream.gd with every chunk made first, in chunk order;
##   streamed   streamed round the walker at the load radius (the chunks within it of each start
##              made first, as the sheet does), loading and unloading as it goes, in lockstep; and
##              once more, `streamed again`.
## Checks: `again` ends bit-equal to `whole` (the same bodies entering a fresh space in the same
## order walk the same; into a space another build had left, border `bank_corner` stopped 40.7 m
## off); `streamed again` bit-equal to `streamed` (a streamed walk is the same on every run);
## `streamed` has `whole`'s outcome and blocker, and its stop within STREAM_STOP. `all` is printed,
## not checked: its bodies enter in another order (a chunk's generated meshes with the chunk), which
## alone moves a stop (coast `along_cliff` 0.0017 m; border `bank_corner` 40.7 m with the chunks in
## reverse order).
## --water names the water field as the sheet's `water` option does, so the walker wades and is
## stopped by deep water as on the sheet (without it, depth is 0 everywhere).
## Prints one line per check and exits 0 when all pass, 1 otherwise. --log writes the reports.

const Loader := preload("res://addons/via_loader/loader.gd")
const Stream := preload("res://addons/via_loader/stream.gd")
const Walk := preload("res://walk.gd")

## Ours: a streamed walk meets only the bodies near it, entered in the order it reached them, and
## Godot's physics resolves a capsule pressed on two bodies by the order they are in the space: on
## atlas `example.church_turn` the whole and streamed trails first part where the walker crosses
## the seam of two chunks' ground bodies (y 1024 m, 14.5 m into the walk) by 0.35 mm. A difference
## that small may still move the frame on which the walker meets its target's body (or the frame
## its stall is judged) by one: there the streamed walk arrived one frame sooner (walked 56.564 m
## against 56.587, 0.023 m, one frame's pace) and stopped 0.0279 m away, a frame's pace along and
## the slide along the wall across. So a stop may lie up to two walk frames' pace (1.4 m/s over
## 1/60 s, twice: 0.047 m) from the whole walk's, and the walked lengths differ by at most one
## frame's pace beyond that rounding (STREAM_WALKED), and the walk is the same walk.
const STREAM_STOP := 2.0 * Walk.SPEED * Walk.DT
const STREAM_WALKED := 1.5 * Walk.SPEED * Walk.DT
var failed := 0
var ran := 0


func _initialize() -> void:
	_run.call_deferred()


func check(what: String, ok: bool, detail := "") -> void:
	ran += 1
	if not ok:
		failed += 1
	print("%s %s%s" % ["ok  " if ok else "FAIL", what, "" if detail == "" else ": " + detail])


var opts := {}


func walks(dir: String, radius: float, only: String, all := false) -> Dictionary:
	## name -> walk report (without its trail); streamed when radius > 0, with every chunk made
	## first when `all`.
	var vp := SubViewport.new()
	vp.own_world_3d = true   # a fresh physics space
	get_root().add_child(vp)
	var loader := Loader.new()
	var root := loader.prepare(dir, opts) if radius > 0.0 else loader.make(dir, opts)
	vp.add_child(root)
	var stream = Stream.new(loader, root, radius) if radius > 0.0 else null
	if all:
		stream.fill(Vector2.ZERO, INF)
	var unbodied: Array = loader.solid_without_body()
	var cams := root.get_node("bookmarks").get_children()
	cams.sort_custom(func(a, b): return String(a.get_meta("feature")) < String(b.get_meta("feature")))
	var out := {}
	for cam in cams:
		var name := String(cam.get_meta("feature"))
		if only != "" and name != only:
			continue
		var e: Array = cam.get_meta("eye")
		var t: Dictionary = cam.get_meta("toward")
		if stream != null and not all:
			stream.update(Vector2(e[0], e[1]))
			stream.fill(Vector2(e[0], e[1]), stream.load_radius)
		var r: Dictionary = await Walk.walk(root, loader, Vector3(e[0], e[1], e[2]), Vector3(t.at[0], t.at[1], t.at[2]),
			String(t.name), unbodied, cam.get_meta("along", []), null if all else stream)
		r.erase("trail")
		r.erase("facing")
		out[name] = r
	if stream != null:
		stream.end()
	vp.free()
	return out


static func apart(a: Dictionary, b: Dictionary) -> float:
	if b.is_empty():
		return INF
	return Vector3(a.stop[0], a.stop[1], a.stop[2]).distance_to(Vector3(b.stop[0], b.stop[1], b.stop[2]))


func bit_equal(what: String, a: Dictionary, b: Dictionary) -> void:
	var differ: Array = []
	for name in a:
		if JSON.stringify(a[name]) != JSON.stringify(b.get(name)):
			differ.append("%s (%.4f m)" % [name, apart(a[name], b.get(name, {}))])
	check("%s: every walk's report bit-equal" % what, differ.is_empty() and a.size() == b.size(),
		"%d walks; differ: %s" % [a.size(), ", ".join(differ)])


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var dir := ""
	var radius := Stream.LOAD
	var only := ""
	var log_file := ""
	var k := 0
	while k < args.size():
		match args[k]:
			"--build": dir = args[k + 1]
			"--radius": radius = float(args[k + 1])
			"--only": only = args[k + 1]
			"--log": log_file = args[k + 1]
			"--water": Loader.parse_option(opts, "water", args[k + 1])
		k += 2
	if dir == "":
		printerr("test_stream_walks: --build DIR is required")
		quit(2)
		return
	var whole := await walks(dir, 0.0, only)
	var again := await walks(dir, 0.0, only)
	var all := await walks(dir, radius, only, true)
	var streamed := await walks(dir, radius, only)
	var streamed_again := await walks(dir, radius, only)
	check("the build has bookmarks to walk from", not whole.is_empty(), "%d" % whole.size())
	bit_equal("whole, loaded again into a fresh space", whole, again)
	var all_most := 0.0
	for name in whole:
		all_most = maxf(all_most, apart(whole[name], all.get(name, {})))
	print("streamed with every chunk made first (a chunk's meshes with it): the farthest stop from whole %.4f m" % all_most)
	bit_equal("streamed, walked again", streamed, streamed_again)
	var most := 0.0
	for name in whole:
		var a: Dictionary = whole[name]
		var b: Dictionary = streamed.get(name, {})
		var d := apart(a, b)
		most = maxf(most, d)
		check("%s: streamed has the whole walk's outcome and blocker, its stop within %.3f m, its length within %.3f m" % [name, STREAM_STOP, STREAM_WALKED],
			not b.is_empty() and a.outcome == b.outcome and a.by == b.by and d <= STREAM_STOP
			and absf(a.walked - b.get("walked", INF)) <= STREAM_WALKED,
			"%s by %s, %s by %s, stops %.4f m apart, walked %.3f and %.3f m" % [a.outcome, a.by, b.get("outcome"), b.get("by"),
				d, a.walked, b.get("walked", -1.0)])
	print("streamed against whole: the farthest stop %.4f m" % most)
	if log_file != "":
		var f := FileAccess.open(log_file, FileAccess.WRITE)
		f.store_string(JSON.stringify({"whole": whole, "streamed": streamed, "radius": radius, "farthest_m": most}, "  ", true))
		f.close()
	print("%s: %d check(s) failed" % ["PASS" if failed == 0 else "FAIL", failed])
	quit(0 if failed == 0 else 1)
