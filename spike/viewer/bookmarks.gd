extends RefCounted
## Bookmarks: the views a world names. Aims the camera at one, applies its cutaway, and answers
## the questions the modes ask about where its eye stands.

const WorldData := preload("res://world_data.gd")
const Built := preload("res://built.gd")

const DEFAULT_FOV := 55.0

var world: WorldData
var cam: Camera3D
var built: Built
var list: Array = []


func _init(world_data: WorldData, camera: Camera3D, built_parts: Built) -> void:
	world = world_data
	cam = camera
	built = built_parts
	list = world.doc["bookmarks"]


func aim(b: Dictionary) -> void:
	var e: Array = b["eye"]
	var t: Array = b["target"]
	cam.position = WorldData.g3(e[0], e[1], e[2])
	cam.look_at(WorldData.g3(t[0], t[1], t[2]), Vector3.UP)
	cam.fov = float(b.get("fov", DEFAULT_FOV))
	built.cut_above(b.get("cut_z"))


func clear_cutaway() -> void:
	built.cut_above(null)


func is_cutaway(b: Dictionary) -> bool:
	return b.get("cut_z") != null


func eye_above_ground(b: Dictionary) -> float:
	## How far the eye stands over the terrain; negative under it.
	return float(b["eye"][2]) - world.h_at(b["eye"][0], b["eye"][1])
