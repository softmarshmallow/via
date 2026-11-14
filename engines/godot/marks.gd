extends RefCounted
## The hand edits marked in the sheet tool's renders (spec/build.md 15.1). Read from a build's
## change report (`changes.json`, build.md 11.2), never from world text: an instance entry whose
## causes name a `pin.<id>` and that was added is the pin's instance; one removed whose cause is a
## `clear.<id>` was cleared, and one removed by a `pin.<id>` gave way to that pin.
##
## Drawn unshaded, in colours no pack material or primitive uses, so they read at any light:
##   pin       a PIN_COLOUR pole POLE metres tall from the ground at the instance, a ball on top
##             (seen from above), and a ring;
##   cleared   a CLEAR_COLOUR ring and a cross on the ground where the removed instance stood, and
##             a square post CLEAR_POST metres tall (no ball: a pin's pole is round and taller);
##   gave way  as cleared, in PIN_COLOUR, where an instance a pin removed stood.
## The same marks go into the before and after renders, so the pair shows the edit's place in both.

const PIN_COLOUR := Color(1.0, 0.15, 0.85)
const CLEAR_COLOUR := Color(0.1, 0.85, 1.0)
const POLE := 9.0
const CLEAR_POST := 2.5
const RING := 1.4
const LIFT := 0.12


## { "against", "pins": [{ "name", "at", "cause" }], "cleared": [{ "name", "at", "cause" }] }
## from `<dir>/changes.json`; empty lists when it has no baseline or no hand edit.
static func read(dir: String) -> Dictionary:
	var out := {"against": null, "pins": [], "cleared": []}
	var path := dir + "/changes.json"
	if not FileAccess.file_exists(path):
		return out
	var doc = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not doc is Dictionary or not doc.get("entries") is Array:
		return out
	out.against = doc.get("against")
	for e in doc.entries:
		if not e is Dictionary or e.get("table") != "instance":
			continue
		var causes: Array = []
		for c in (e.get("causes") if e.get("causes") is Array else []):
			causes.append(String(c.get("cause", "")))
		var what: Array = e.get("what", [])
		var pin := causes.filter(func(c): return c.begins_with("pin."))
		var clear := causes.filter(func(c): return c.begins_with("clear."))
		if "added" in what and not pin.is_empty() and e.get("after") is Dictionary:
			out.pins.append({"name": e.name, "at": e.after.at, "cause": pin[0]})
		elif "removed" in what and e.get("before") is Dictionary and not (clear.is_empty() and pin.is_empty()):
			out.cleared.append({"name": e.name, "at": e.before.at, "cause": clear[0] if not clear.is_empty() else pin[0]})
	return out


static func count(edits: Dictionary) -> int:
	return edits.pins.size() + edits.cleared.size()


## A Node3D named `marks` holding every mark, laid on the loader's ground.
static func node(edits: Dictionary, loader) -> Node3D:
	var holder := Node3D.new()
	holder.name = "marks"
	for p in edits.pins:
		var at: Array = p.at
		var z: float = loader.ground_at(at[0], at[1]) if loader.has_ground() else float(at[2])
		var pole := MeshInstance3D.new()
		var cyl := CylinderMesh.new()
		cyl.top_radius = 0.12
		cyl.bottom_radius = 0.12
		cyl.height = POLE
		pole.mesh = cyl
		pole.material_override = _flat(PIN_COLOUR)
		pole.position = Vector3(at[0], z + POLE / 2.0, -at[1])
		pole.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		pole.name = "pin_" + String(p.name).replace(".", "-")
		holder.add_child(pole)
		var head := MeshInstance3D.new()   # seen from above, where a pole is a dot
		var ball := SphereMesh.new()
		ball.radius = 0.9
		ball.height = 1.8
		head.mesh = ball
		head.material_override = _flat(PIN_COLOUR)
		head.position = Vector3(at[0], z + POLE, -at[1])
		head.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		holder.add_child(head)
		holder.add_child(_ring(Vector3(at[0], z + LIFT, -at[1]), PIN_COLOUR, false))
	for c in edits.cleared:
		var at: Array = c.at
		var z: float = loader.ground_at(at[0], at[1]) if loader.has_ground() else float(at[2])
		var colour := CLEAR_COLOUR if String(c.cause).begins_with("clear.") else PIN_COLOUR
		holder.add_child(_ring(Vector3(at[0], z + LIFT, -at[1]), colour, true))
		var post := MeshInstance3D.new()   # seen at eye height over grass and through trees
		var box := BoxMesh.new()
		box.size = Vector3(0.15, CLEAR_POST, 0.15)
		post.mesh = box
		post.material_override = _flat(colour)
		post.position = Vector3(at[0], z + CLEAR_POST / 2.0, -at[1])
		post.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		holder.add_child(post)
	return holder


static func _flat(c: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.albedo_color = c
	m.cull_mode = BaseMaterial3D.CULL_DISABLED
	return m


static func _ring(at: Vector3, colour: Color, cross: bool) -> Node3D:
	var n := Node3D.new()
	n.position = at
	var ring := MeshInstance3D.new()
	var t := TorusMesh.new()
	t.inner_radius = RING - 0.18
	t.outer_radius = RING
	ring.mesh = t
	ring.material_override = _flat(colour)
	ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	n.add_child(ring)
	if cross:
		for a in [45.0, -45.0]:
			var bar := MeshInstance3D.new()
			var b := BoxMesh.new()
			b.size = Vector3(RING * 2.0, 0.08, 0.2)
			bar.mesh = b
			bar.material_override = _flat(colour)
			bar.rotation_degrees = Vector3(0, a, 0)
			bar.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			n.add_child(bar)
	return n
