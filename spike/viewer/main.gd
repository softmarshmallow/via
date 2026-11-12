extends Node3D
## Spike viewer. Loads a compiled world and either renders its bookmarks or lets you walk it.
##   -- --build=DIR [--out=DIR] [--shots [--only=a,b] | --bench | --walktest | --mapshot]
## This script reads the arguments, loads the world, builds the scene from its parts and picks the mode.

const WorldData := preload("res://world_data.gd")
const SkyRig := preload("res://sky.gd")
const Terrain := preload("res://terrain.gd")
const Instances := preload("res://instances.gd")
const Built := preload("res://built.gd")
const Grass := preload("res://grass.gd")
const Bookmarks := preload("res://bookmarks.gd")
const Walk := preload("res://walk.gd")
const Review := preload("res://review.gd")

## Shots are drawn off screen at this size, whatever the window is.
const SHOT_SIZE := Vector2i(1920, 1080)
## While shooting, the window is a small sliver parked in the screen's far corner, out of the way.
const PARKED_SIZE := Vector2i(640, 360)
const PARKED_INSET := Vector2i(24, 24)
const WALK_SIZE := Vector2i(1600, 900)
const CAM_NEAR_M := 0.15
const CAM_FAR_M := 1600.0

var build_dir := ""
var out_dir := ""
var shots := false
var bench := false
var walktest := false
var mapshot := false
var only: PackedStringArray = []

var world: WorldData
var stage: Node3D
var picture: SubViewport      # where shots are drawn; null in the other modes
var review: Review


func _ready() -> void:
	if not _read_arguments():
		get_tree().quit(1)
		return
	world = WorldData.new()
	if not world.load_from(build_dir):
		push_error("no compiled world at " + build_dir)
		get_tree().quit(1)
		return
	if walktest and world.doc["name"] != Review.WALKTEST_WORLD:
		printerr("WALKTEST REFUSED: the walk test is written for the %s world, and this build is %s" % [Review.WALKTEST_WORLD, world.doc["name"]])
		get_tree().quit(1)
		return
	_open_stage()

	# volumes borrow the terrain's material, so that comes first
	SkyRig.build(world, stage, shots)
	var terrain := Terrain.new(world, stage, shots)
	var built := Built.new(world, stage, not shots and not bench)
	terrain.build_material()
	built.build_volumes(terrain.material)
	terrain.build_ground()
	terrain.build_water()
	var instances := Instances.new(world, stage, shots)
	instances.build()
	built.build_meshes()
	built.build_lights()
	var grass := Grass.new(world, stage, shots)
	var cam := Camera3D.new()
	cam.near = CAM_NEAR_M
	cam.far = CAM_FAR_M
	stage.add_child(cam)
	cam.make_current()
	var marks := Bookmarks.new(world, cam, built)
	review = Review.new(get_tree(), world, marks, grass, out_dir)

	if shots:
		await review.shoot(picture, only)
		get_tree().quit()
		return
	if bench:      # no walking body, so nothing here listens to the keyboard or mouse
		await review.bench(get_window())
		get_tree().quit()
		return
	var walk := Walk.new(world, stage, cam, marks, instances, grass)
	add_child(walk)
	walk.start()
	if mapshot:      # capture the two map views and leave
		await review.mapshot(get_window(), walk)
		get_tree().quit()
	elif walktest:
		var passed: bool = await review.walktest(walk)
		get_tree().quit(0 if passed else 1)


func _read_arguments() -> bool:
	## False when the arguments ask for more than one of the modes that run by themselves.
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--build="):
			build_dir = a.trim_prefix("--build=")
		elif a.begins_with("--out="):
			out_dir = a.trim_prefix("--out=")
		elif a.begins_with("--only="):
			only = a.trim_prefix("--only=").split(",")
		elif a == "--shots":
			shots = true
		elif a == "--bench":
			bench = true
		elif a == "--walktest":
			walktest = true
		elif a == "--mapshot":
			mapshot = true
	if build_dir == "":
		build_dir = ProjectSettings.globalize_path("res://../build/vale")
	if int(shots) + int(bench) + int(walktest) + int(mapshot) > 1:
		printerr("ERROR: --shots, --bench, --walktest and --mapshot cannot be combined; give one")
		return false
	return true


func _open_stage() -> void:
	## Shots draw into an off-screen viewport with its own 3D world; the other modes use the window.
	var win := get_window()
	stage = Node3D.new()
	if shots:
		win.unfocusable = true
		win.borderless = true
		win.always_on_top = true
		win.size = PARKED_SIZE
		win.position = DisplayServer.screen_get_position() + DisplayServer.screen_get_size() - PARKED_INSET
		picture = SubViewport.new()
		picture.size = SHOT_SIZE
		picture.msaa_3d = Viewport.MSAA_4X
		picture.render_target_update_mode = SubViewport.UPDATE_ALWAYS
		picture.own_world_3d = true
		add_child(picture)
		picture.add_child(stage)
	else:
		get_viewport().msaa_3d = Viewport.MSAA_2X
		win.size = WALK_SIZE
		win.title = "%s (worldc spike)" % world.doc["name"]
		add_child(stage)
