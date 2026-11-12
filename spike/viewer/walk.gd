extends Node
## Walking a world: the body and its collision, keyboard and mouse, fly mode, bookmark jumps, the
## HUD and the minimap. The node exists only in the modes that walk.

const WorldData := preload("res://world_data.gd")
const Instances := preload("res://instances.gd")
const Grass := preload("res://grass.gd")
const Bookmarks := preload("res://bookmarks.gd")
const Minimap := preload("res://minimap.gd")

const EYE_HEIGHT_M := 1.75
const BODY_RADIUS_M := 0.3
const BODY_HEIGHT_M := 1.7
const MAX_SLOPE_DEG := 52.0
const FLOOR_SNAP_M := 0.45
const GRAVITY := 20.0
const WALK_SPEED := 4.2
const RUN_SPEED := 11.0
const FLY_SPEED_FACTOR := 4.0
const JUMP_SPEED := 6.2
## Flying has gears: each press of Up or + doubles the speed, Down or - halves it.
const FLY_GEAR_MIN := 0.25
const FLY_GEAR_MAX := 64.0
const MOUSE_RAD_PER_PX := 0.0025
const PITCH_LIMIT := 1.5
## Bookmarks are reached with the digit keys 1 to 9.
const DIGIT_KEYS := 9
## The body may not come nearer than this to the world's edge.
const EDGE_MARGIN_M := 1.0
## A bookmark this far above the ground is a view from the air: arrive flying.
const FLY_ABOVE_M := 6.0
## Start at the first bookmark whose eye is no higher than this, so the walk begins on foot.
const ON_FOOT_BELOW_M := 4.0
## An eye well off the terrain but below this height, in a world with rooms, is taken to be indoors.
const INDOOR_OFF_GROUND_M := 2.5
const INDOOR_BELOW_Z := 40.0
## Under an overhang the bed is the floor: used once the body is this far below the upper surface.
const UNDER_OVERHANG_M := 2.2
## The body stands this deep in water before the water holds it up.
const WADE_DEPTH_M := 0.9
## Where a volume or interior stands, the terrain gives no floor at all.
const NO_FLOOR := -1000.0
const GRASS_TILES_PER_FRAME := 3

var world: WorldData
var stage: Node3D
var cam: Camera3D
var marks: Bookmarks
var instances: Instances
var grass: Grass

var body: CharacterBody3D
var yaw := 0.0
var pitch := -0.1
var fly := false
var fly_gear := 1.0
var scripted := false      # a scripted check is driving the body; the keyboard stands aside
var _vel_y := 0.0
var _grounded := false
var _hud: Label
var _minimap: Minimap


func _init(world_data: WorldData, stage_node: Node3D, camera: Camera3D, bookmarks: Bookmarks,
		placed: Instances, grass_kit: Grass) -> void:
	world = world_data
	stage = stage_node
	cam = camera
	marks = bookmarks
	instances = placed
	grass = grass_kit


func start() -> void:
	body = CharacterBody3D.new()
	var capsule := CapsuleShape3D.new()
	capsule.radius = BODY_RADIUS_M
	capsule.height = BODY_HEIGHT_M
	var cs := CollisionShape3D.new()
	cs.shape = capsule
	cs.position = Vector3(0, BODY_HEIGHT_M * 0.5, 0)
	body.add_child(cs)
	body.floor_max_angle = deg_to_rad(MAX_SLOPE_DEG)
	body.floor_snap_length = FLOOR_SNAP_M
	stage.add_child(body)
	cam.get_parent().remove_child(cam)
	body.add_child(cam)
	instances.add_colliders()
	var first: Dictionary = marks.list[0] if not marks.list.is_empty() else {}
	for b in marks.list:      # prefer to start on foot
		if marks.eye_above_ground(b) < ON_FOOT_BELOW_M and not marks.is_cutaway(b):
			first = b
			break
	if not first.is_empty():
		go_to(first)
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	var layer := CanvasLayer.new()
	add_child(layer)
	_hud = Label.new()
	_hud.position = Vector2(14, 10)
	_hud.add_theme_color_override("font_color", Color(1, 1, 1, 0.9))
	_hud.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.8))
	_hud.add_theme_constant_override("outline_size", 4)
	layer.add_child(_hud)
	_add_minimap(layer)


func _add_minimap(layer: CanvasLayer) -> void:
	## The painted map if the build has one, else the compiler's plain base. Icons come from data.
	var dir := world.build_dir + "/map"
	var text := FileAccess.get_file_as_string(dir + "/map.json")
	if text == "":
		return
	var picture := ""
	for candidate in ["styled.png", "styled.jpg", "base.png"]:
		if FileAccess.file_exists(dir + "/" + candidate):
			picture = dir + "/" + candidate
			break
	if picture == "":
		return
	var img := Image.load_from_file(picture)
	img.generate_mipmaps()
	_minimap = Minimap.new()
	_minimap.set_anchors_preset(Control.PRESET_FULL_RECT)
	_minimap.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_minimap.tex = ImageTexture.create_from_image(img)
	_minimap.meta = JSON.parse_string(text)
	layer.add_child(_minimap)


func show_full_map(on: bool) -> void:
	if _minimap != null:
		_minimap.full = on
		_minimap.queue_redraw()


func go_to(b: Dictionary) -> void:
	## Stand where a bookmark stands, looking where it looks.
	cam.top_level = true
	marks.aim(b)
	var fwd := -cam.global_transform.basis.z
	yaw = atan2(-fwd.x, -fwd.z)
	pitch = asin(clampf(fwd.y, -1.0, 1.0))
	var eye := cam.global_position
	cam.top_level = false
	marks.clear_cutaway()      # on foot nothing is cut away
	body.global_position = eye - Vector3(0, EYE_HEIGHT_M, 0)
	cam.position = Vector3(0, EYE_HEIGHT_M, 0)
	_vel_y = 0.0
	var above := body.global_position.y - world.h_at(body.global_position.x, -body.global_position.z)
	fly = above > FLY_ABOVE_M and not marks.is_cutaway(b) and not _indoors(b)


func _indoors(b: Dictionary) -> bool:
	# a guess from heights alone: it knows nothing of which room, if any, the eye is actually in
	return absf(marks.eye_above_ground(b)) > INDOOR_OFF_GROUND_M and float(b["eye"][2]) < INDOOR_BELOW_Z \
		and not world.doc.get("rooms", {}).is_empty()


func place(at: Vector3) -> void:
	## Put the body somewhere, at rest.
	body.global_position = at
	_vel_y = 0.0


func step_scripted(flat: Vector3) -> void:
	## One physics tick of walking with a given horizontal velocity, for scripted checks.
	_vel_y -= GRAVITY / Engine.physics_ticks_per_second
	body.velocity = Vector3(flat.x, _vel_y, flat.z)
	body.move_and_slide()
	_vel_y = body.velocity.y


func _floor_under(p: Vector3) -> float:
	## The terrain under a point; beneath an overhang or in a cave, the lowest floor.
	var wx := p.x
	var wy := -p.z
	if not world.hole.is_empty() and world.sample(world.hole, wx, wy) > 0.5:
		return NO_FLOOR      # a volume or an interior stands here: its own collision mesh is the floor
	var top := world.h_at(wx, wy)
	if not world.bed.is_empty() and p.y < top - UNDER_OVERHANG_M:
		top = world.sample(world.bed, wx, wy)
	if not world.water.is_empty():
		top = maxf(top, world.sample(world.water, wx, wy) - WADE_DEPTH_M)
	return top


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		yaw -= event.relative.x * MOUSE_RAD_PER_PX
		pitch = clampf(pitch - event.relative.y * MOUSE_RAD_PER_PX, -PITCH_LIMIT, PITCH_LIMIT)
	elif event is InputEventKey and event.pressed and not event.echo:
		match event.physical_keycode:
			KEY_ESCAPE:
				Input.mouse_mode = Input.MOUSE_MODE_VISIBLE if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED else Input.MOUSE_MODE_CAPTURED
			KEY_F:
				fly = not fly
			KEY_UP, KEY_EQUAL, KEY_KP_ADD:
				fly_gear = minf(fly_gear * 2.0, FLY_GEAR_MAX)
			KEY_DOWN, KEY_MINUS, KEY_KP_SUBTRACT:
				fly_gear = maxf(fly_gear * 0.5, FLY_GEAR_MIN)
			KEY_M:
				if _minimap != null:
					show_full_map(not _minimap.full)
			KEY_Q:
				if Input.mouse_mode == Input.MOUSE_MODE_VISIBLE:
					get_tree().quit()
			_:
				var n: int = event.physical_keycode - KEY_1
				if n >= 0 and n < mini(marks.list.size(), DIGIT_KEYS):      # the digit keys only: letters have higher key codes
					go_to(marks.list[n])
	elif event is InputEventMouseButton and event.pressed:
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED


func _physics_process(delta: float) -> void:
	if body == null or scripted:
		return
	var move := Vector3.ZERO
	if Input.is_physical_key_pressed(KEY_W):
		move.z -= 1.0
	if Input.is_physical_key_pressed(KEY_S):
		move.z += 1.0
	if Input.is_physical_key_pressed(KEY_A):
		move.x -= 1.0
	if Input.is_physical_key_pressed(KEY_D):
		move.x += 1.0
	var speed := RUN_SPEED if Input.is_physical_key_pressed(KEY_SHIFT) else WALK_SPEED
	if fly:      # no collision: pass through anything
		var dir := cam.global_transform.basis * move
		if Input.is_physical_key_pressed(KEY_SPACE):
			dir.y += 1.0
		if Input.is_physical_key_pressed(KEY_CTRL):
			dir.y -= 1.0
		if dir.length() > 0.0:
			body.global_position += dir.normalized() * speed * FLY_SPEED_FACTOR * fly_gear * delta
		_vel_y = 0.0
	else:
		var flat := Basis(Vector3.UP, yaw) * move
		if flat.length() > 0.0:
			flat = flat.normalized() * speed
		_vel_y -= GRAVITY * delta
		if _grounded and Input.is_physical_key_pressed(KEY_SPACE):
			_vel_y = JUMP_SPEED
		body.velocity = Vector3(flat.x, _vel_y, flat.z)
		body.move_and_slide()
		_grounded = body.is_on_floor()
		_vel_y = body.velocity.y
		var ground := _floor_under(body.global_position)      # the terrain is a floor without a mesh
		if body.global_position.y <= ground + 0.02:
			body.global_position.y = ground
			_vel_y = maxf(_vel_y, 0.0)
			_grounded = true
	var p := body.global_position
	p.x = clampf(p.x, world.x_min + EDGE_MARGIN_M, world.x_max - EDGE_MARGIN_M)
	p.z = clampf(p.z, -world.y_max + EDGE_MARGIN_M, -world.y_min - EDGE_MARGIN_M)
	body.global_position = p


func _process(_delta: float) -> void:
	if body == null:
		return
	cam.rotation = Vector3(pitch, yaw, 0)
	var p := body.global_position
	grass.grow_around(p.x, -p.z, GRASS_TILES_PER_FRAME)
	if _minimap != null:
		_minimap.player = Vector2(p.x, -p.z)
		_minimap.heading = yaw
		_minimap.queue_redraw()
	_hud.text = "%s   x %.0f  y %.0f  z %.1f   %s   %d fps\nWASD move, mouse look, Shift run, Space jump, F fly/walk, Up/Down or +/- fly speed, M map, 1-%d bookmarks, Esc release mouse (then Q quits)" % [
		world.doc["name"], p.x, -p.z, p.y, ("fly x%s, %.0f m/s (no collision)" % [str(fly_gear), WALK_SPEED * FLY_SPEED_FACTOR * fly_gear]) if fly else "walk", Engine.get_frames_per_second(), mini(marks.list.size(), DIGIT_KEYS)]
