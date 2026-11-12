extends Control
## The player map at run time. The picture is the painted map; everything on top of it (the player,
## icons, labels) is drawn from the compiled map data through the world-to-map transform.

const FRAME := Color(0.16, 0.11, 0.08, 0.95)
const INK := Color(0.16, 0.1, 0.08)
const PAPER := Color(0.96, 0.92, 0.82)
const LABEL_INK := Color(0.25, 0.16, 0.12)
const LANDMARK := Color(0.75, 0.2, 0.16)
const PLACE := Color(0.98, 0.94, 0.84)
const OFF_MAP := Color(0.55, 0.50, 0.42)      # shows where the corner view runs past the picture
const PLAYER := Color(0.85, 0.15, 0.12)
## The corner view: a square of this many pixels, this far from the window's top right corner.
const BOX_PX := 250.0
const BOX_INSET_PX := 18.0
## The full map fills this share of the window.
const FULL_SHARE := 0.88
const HINT := "N up   M: full map"

var tex: Texture2D
var meta: Dictionary = {}
var player := Vector2.ZERO      # world x (east), y (north)
var heading := 0.0              # camera yaw, radians
var full := false
var radius_m := 110.0


func _map_px(p: Vector2) -> Vector2:
	var t: Dictionary = meta["transform"]
	var e: Array = t["extent"]
	return Vector2((p.x - float(e[0])) * float(t["px_per_m"]), float(t["height_px"]) - (p.y - float(e[1])) * float(t["px_per_m"]))


func _icon_px(icon: Dictionary) -> Vector2:
	return _map_px(Vector2(icon["pos"][0], icon["pos"][1]))


func _arrow(at: Vector2, size: float) -> void:
	var fwd := Vector2(-sin(heading), -cos(heading))
	var side := Vector2(-fwd.y, fwd.x)
	var tip := at + fwd * size
	var left := at - fwd * size * 0.7 + side * size * 0.7
	var right := at - fwd * size * 0.7 - side * size * 0.7
	draw_colored_polygon(PackedVector2Array([tip, left, right]), PLAYER)
	draw_polyline(PackedVector2Array([tip, left, right, tip]), Color(1, 1, 1), 2.0)


func _outlined(at: Vector2, text: String, size: int, outline: int, outline_col: Color, col: Color) -> void:
	var font := ThemeDB.fallback_font
	draw_string_outline(font, at, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, outline, outline_col)
	draw_string(font, at, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, col)


func _dot(at: Vector2, radius: float, icon: Dictionary) -> void:
	draw_circle(at, radius + 2.0, INK)
	draw_circle(at, radius, LANDMARK if icon["type"] == "landmark" else PLACE)


func _draw() -> void:
	if tex == null or meta.is_empty():
		return
	if full:
		_draw_full()
	else:
		_draw_corner()


func _draw_full() -> void:
	## The whole map, centred, with its labels.
	var t: Dictionary = meta["transform"]
	var map_size := Vector2(float(t["width_px"]), float(t["height_px"]))
	var vp := get_viewport_rect().size
	var s := minf(vp.x * FULL_SHARE / map_size.x, vp.y * FULL_SHARE / map_size.y)
	var size := map_size * s
	var origin := (vp - size) * 0.5
	draw_rect(Rect2(origin - Vector2(10, 10), size + Vector2(20, 20)), FRAME)
	draw_texture_rect(tex, Rect2(origin, size), false)
	for lab in meta.get("labels", []):
		var lp: Vector2 = origin + _map_px(Vector2(lab["pos"][0], lab["pos"][1])) * s
		_outlined(lp - Vector2(60, 0), String(lab["text"]).to_upper(), 22, 5, PAPER, LABEL_INK)
	for icon in meta.get("icons", []):
		var ip: Vector2 = origin + _icon_px(icon) * s
		_dot(ip, 5.0, icon)
		if String(icon.get("label", "")) != "":
			_outlined(ip + Vector2(10, 5), icon["label"], 15, 4, PAPER, INK)
	_arrow(origin + _map_px(player) * s, 11.0)


func _draw_corner() -> void:
	## A north-up square around the player, `radius_m` to each side.
	var t: Dictionary = meta["transform"]
	var map_size := Vector2(float(t["width_px"]), float(t["height_px"]))
	var tex_scale := tex.get_size() / map_size      # the painted picture need not be the data's size
	var vp := get_viewport_rect().size
	var box := Vector2(BOX_PX, BOX_PX)
	var origin := Vector2(vp.x - BOX_PX - BOX_INSET_PX, BOX_INSET_PX)
	var centre := _map_px(player)
	var half := radius_m * float(t["px_per_m"])
	var src := Rect2((centre - Vector2(half, half)) * tex_scale, Vector2(half, half) * 2.0 * tex_scale)
	draw_rect(Rect2(origin - Vector2(5, 5), box + Vector2(10, 10)), FRAME)
	draw_rect(Rect2(origin, box), OFF_MAP)
	draw_texture_rect_region(tex, Rect2(origin, box), src)
	var k := BOX_PX / (half * 2.0)
	for icon in meta.get("icons", []):
		var rel: Vector2 = (_icon_px(icon) - centre) * k
		if absf(rel.x) < BOX_PX * 0.5 - 6.0 and absf(rel.y) < BOX_PX * 0.5 - 6.0:
			_dot(origin + box * 0.5 + rel, 4.0, icon)
	_arrow(origin + box * 0.5, 9.0)
	_outlined(origin + Vector2(6, BOX_PX - 8), HINT, 12, 3, Color(0, 0, 0, 0.7), Color(1, 1, 1, 0.9))
