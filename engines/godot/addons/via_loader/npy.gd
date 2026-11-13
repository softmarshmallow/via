extends RefCounted
## Array files (spec/build.md 3.2, protocol.md 4.2): NumPy .npy 1.0, C order, little-endian, in the
## subset the build writes: descr `<f4`, `<i4` or `|u1`, one to three dimensions.
## read() returns { "error": "" or why, "descr", "shape": Array[int], "bytes": PackedByteArray },
## `bytes` being the data alone. It never panics; a bad file is reported by the caller as
## load.bad_file with the reason given here.

const MAGIC := [0x93, 0x4E, 0x55, 0x4D, 0x50, 0x59]
const ITEM := {"<f4": 4, "<i4": 4, "|u1": 1}


static func read(path: String) -> Dictionary:
	var out := {"error": "", "descr": "", "shape": [], "bytes": PackedByteArray()}
	if not FileAccess.file_exists(path):
		out.error = "missing"
		return out
	var raw := FileAccess.get_file_as_bytes(path)
	if raw.size() < 10:
		out.error = "shorter than an array file's header"
		return out
	for n in MAGIC.size():
		if raw[n] != MAGIC[n]:
			out.error = "not an array file (no .npy magic)"
			return out
	if raw[6] != 1 or raw[7] != 0:
		out.error = "format version %d.%d, not 1.0" % [raw[6], raw[7]]
		return out
	var hlen := raw[8] | (raw[9] << 8)
	var start := 10 + hlen
	if start > raw.size() or start % 64 != 0:
		out.error = "header length %d does not end on a multiple of 64 bytes" % hlen
		return out
	var text := raw.slice(10, start).get_string_from_ascii()
	var descr := _between(text, "'descr': '", "'")
	if not ITEM.has(descr):
		out.error = "descr '%s' is not one the build writes" % descr
		return out
	if text.find("'fortran_order': False") < 0:
		out.error = "not C order"
		return out
	var dims := _between(text, "'shape': (", ")")
	var shape: Array = []
	for part in dims.split(",", false):
		var p := part.strip_edges()
		if not p.is_valid_int():
			out.error = "shape (%s) is not a list of integers" % dims
			return out
		shape.append(p.to_int())
	if shape.is_empty() or shape.size() > 3:
		out.error = "%d dimensions; the build writes one to three" % shape.size()
		return out
	var count := 1
	for d in shape:
		count *= d
	var want: int = count * ITEM[descr]
	if raw.size() - start != want:
		out.error = "%d data bytes, the header's shape needs %d" % [raw.size() - start, want]
		return out
	out.descr = descr
	out.shape = shape
	out.bytes = raw.slice(start)
	return out


static func expect(arr: Dictionary, descr: String, shape: Array) -> String:
	## "" when the file holds `descr` and `shape`, else what differs.
	if arr.error != "":
		return arr.error
	if arr.descr != descr:
		return "descr %s, expected %s" % [arr.descr, descr]
	if arr.shape != shape:
		return "shape %s, expected %s" % [str(arr.shape), str(shape)]
	return ""


static func floats(arr: Dictionary) -> PackedFloat32Array:
	## Little-endian on every platform Godot ships for; the bytes are read as they lie.
	return arr.bytes.to_float32_array()


static func ints(arr: Dictionary) -> PackedInt32Array:
	return arr.bytes.to_int32_array()


static func _between(text: String, a: String, b: String) -> String:
	var i := text.find(a)
	if i < 0:
		return ""
	i += a.length()
	var j := text.find(b, i)
	return text.substr(i, j - i) if j >= 0 else ""
