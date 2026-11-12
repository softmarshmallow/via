extends RefCounted
## The compiled world in memory: the frame, the rasters as packed arrays, and how to sample them.
## World axes are x east, y north, z up. Godot is X east, Y up, Z south: (x, y, z) -> (x, z, -y).

## The terrain shader blends this many ground materials; their weights travel four to a texture.
const GROUND_LAYERS := 8
const WEIGHTS_PER_TEXTURE := 4
## A tint or colour that changes nothing.
const WHITE_RGB := [1.0, 1.0, 1.0]

var build_dir := ""
var doc: Dictionary = {}       # world.json as written by the compiler
var look: Dictionary = {}      # optional overrides of sky, sun, water and rock
var nx := 0
var ny := 0
var res := 0.5
var x_min := 0.0
var y_min := 0.0
var x_max := 1.0
var y_max := 1.0
var chunk := 0.0
var height := PackedFloat32Array()
var water := PackedFloat32Array()
var grass := PackedFloat32Array()
var bed := PackedFloat32Array()
var hole := PackedFloat32Array()
var w0 := PackedByteArray()
var w1 := PackedByteArray()
var ground_cols: Array[Color] = []


func load_from(dir: String) -> bool:
	## False when the folder holds no compiled world.
	build_dir = dir
	var text := FileAccess.get_file_as_string(dir + "/world.json")
	if text == "":
		return false
	doc = JSON.parse_string(text)
	look = doc.get("look", {})
	var fr: Dictionary = doc["frame"]
	nx = int(fr["nx"])
	ny = int(fr["ny"])
	res = float(fr["resolution"])
	x_min = float(fr["extent"][0])
	y_min = float(fr["extent"][1])
	x_max = float(fr["extent"][2])
	y_max = float(fr["extent"][3])
	chunk = float(fr["chunk"])
	height = _field("height")
	if doc["fields"].has("water"):
		water = _field("water")
	if doc["fields"].has("grass"):
		grass = _field("grass")
	if doc["fields"].has("hole"):
		hole = _field("hole")
	if doc["fields"].has("bed"):
		bed = _field("bed")
	w0 = FileAccess.get_file_as_bytes(dir + "/ground_0.rgba8")
	w1 = FileAccess.get_file_as_bytes(dir + "/ground_1.rgba8")
	for g in doc["ground"]:
		ground_cols.append(rgb(g["color"]))
	return true


func _field(field: String) -> PackedFloat32Array:
	return FileAccess.get_file_as_bytes("%s/%s.f32" % [build_dir, field]).to_float32_array()


static func g3(x: float, y: float, z: float) -> Vector3:
	return Vector3(x, z, -y)


static func rgb(c: Array) -> Color:
	return Color(c[0], c[1], c[2])


func width() -> float:
	return x_max - x_min


func depth() -> float:
	return y_max - y_min


func centre(z: float) -> Vector3:
	## The middle of the world at a given height, in engine axes.
	return g3((x_min + x_max) * 0.5, (y_min + y_max) * 0.5, z)


func extent4() -> Vector4:
	return Vector4(x_min, y_min, x_max, y_max)


func sample(arr: PackedFloat32Array, x: float, y: float) -> float:
	var fx := clampf((x - x_min) / res, 0.0, nx - 1.001)
	var fy := clampf((y - y_min) / res, 0.0, ny - 1.001)
	var i := int(fx)
	var j := int(fy)
	var u := fx - i
	var v := fy - j
	var k := j * nx + i
	return lerpf(lerpf(arr[k], arr[k + 1], u), lerpf(arr[k + nx], arr[k + nx + 1], u), v)


func h_at(x: float, y: float) -> float:
	return sample(height, x, y)


func ground_color(x: float, y: float) -> Color:
	var i := clampi(int(round((x - x_min) / res)), 0, nx - 1)
	var j := clampi(int(round((y - y_min) / res)), 0, ny - 1)
	var k := (j * nx + i) * WEIGHTS_PER_TEXTURE
	var c := Color(0, 0, 0)
	for m in ground_cols.size():
		var w := float(w0[k + m]) if m < WEIGHTS_PER_TEXTURE else float(w1[k + m - WEIGHTS_PER_TEXTURE])
		if w > 0.0:
			c += ground_cols[m] * (w / 255.0)
	return c


func look_color(key: String, fallback: Color) -> Color:
	if not look.has(key):
		return fallback
	return rgb(look[key])


func sea_level(fallback: float) -> float:
	return float(doc.get("params", {}).get("sea", fallback))


func field_texture(arr: PackedFloat32Array) -> ImageTexture:
	return ImageTexture.create_from_image(Image.create_from_data(nx, ny, false, Image.FORMAT_RF, arr.to_byte_array()))


func weight_texture(bytes: PackedByteArray) -> ImageTexture:
	return ImageTexture.create_from_image(Image.create_from_data(nx, ny, false, Image.FORMAT_RGBA8, bytes))
