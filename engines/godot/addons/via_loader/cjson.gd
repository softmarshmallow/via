extends RefCounted
## Canonical JSON (spec/documents.md 8, build.md 3.1): UTF-8, keys in bytewise order, no white
## space, one LF at the end; a double as the shortest decimal that reads back to the same bits, an
## integral value as an integer, never a signed zero. Godot's JSON.stringify does not promise the
## shortest form, so the loader's documents (diagnostics.json, sheet.json) go through this.


static func text(value) -> String:
	return encode(value) + "\n"


static func encode(value) -> String:
	match typeof(value):
		TYPE_NIL:
			return "null"
		TYPE_BOOL:
			return "true" if value else "false"
		TYPE_INT:
			return str(value)
		TYPE_FLOAT:
			return number(value)
		TYPE_STRING, TYPE_STRING_NAME:
			return JSON.stringify(String(value))
		TYPE_DICTIONARY:
			var keys: Array = value.keys().map(func(k): return String(k))
			keys.sort_custom(_less)
			var parts := PackedStringArray()
			for k in keys:
				parts.append(JSON.stringify(k) + ":" + encode(value[k]))
			return "{" + ",".join(parts) + "}"
		_:
			if value is Array or typeof(value) >= TYPE_PACKED_BYTE_ARRAY:
				var parts := PackedStringArray()
				for v in value:
					parts.append(encode(v))
				return "[" + ",".join(parts) + "]"
	return JSON.stringify(str(value))


static func number(x: float) -> String:
	## The shortest decimal that reads back to x's bits, in plain notation (numbers.md 3.4). Godot's
	## own parser and formatter are not correctly rounded at 16 and 17 digits, so the digits are
	## worked out exactly: x and the midpoints to its two neighbours as exact decimals, then the
	## first rounding of x, from 1 significant digit up, that lies strictly between the midpoints
	## (or on one when x's significand is even: ties read back to even).
	if is_nan(x) or is_inf(x):
		return "null"   # never written: the loader's values are finite; kept total, not guessed
	if x == 0.0:
		return "0"
	var neg := x < 0.0
	var bits := PackedFloat64Array([absf(x)]).to_byte_array().decode_s64(0)
	var field := (bits >> 52) & 0x7FF
	var mant := bits & ((1 << 52) - 1)
	var m := mant | (1 << 52) if field > 0 else mant
	var e := (field if field > 0 else 1) - 1075
	# x = 4m * 2^(e-2); the midpoints, in the same unit, are 4m - 2 (or 4m - 1 when the gap below
	# is half as wide, at a power of two) and 4m + 2.
	var lo_n := 4 * m - (1 if mant == 0 and field > 1 else 2)
	var hi_n := 4 * m + 2
	var f := e - 2
	var lo := _decimal(lo_n, f)
	var mid := _decimal(4 * m, f)
	var hi := _decimal(hi_n, f)
	var width := maxi(maxi(lo.size(), mid.size()), hi.size())
	var frac := maxi(0, -f)      # digits after the point in all three
	var xs := _digits(mid, width)
	var los := _digits(lo, width)
	var his := _digits(hi, width)
	var inclusive := m % 2 == 0
	var lead := 0
	while lead < width - 1 and xs[lead] == "0":
		lead += 1
	for d in range(1, width - lead + 1):
		var cand := _round_at(xs, lead + d)
		var above: bool = cand > los or (inclusive and cand == los)
		var below: bool = cand < his or (inclusive and cand == his)
		if above and below:
			return ("-" if neg else "") + _place(cand, frac)
	return ("-" if neg else "") + _place(xs, frac)


static func _decimal(n: int, f: int) -> PackedByteArray:
	## n * 2^f as an exact decimal integer of its digits, little-endian, in units of 10^-max(0, -f).
	var d := PackedByteArray()
	var v := n
	while v > 0:
		d.append(v % 10)
		v /= 10
	var times := 2 if f > 0 else 5
	for k in absi(f):
		var carry := 0
		for i in d.size():
			var t: int = d[i] * times + carry
			d[i] = t % 10
			carry = t / 10
		while carry > 0:
			d.append(carry % 10)
			carry /= 10
	return d


static func _digits(d: PackedByteArray, width: int) -> String:
	## Big-endian digit text, padded with zeros on the left to `width`.
	var s := ""
	for i in range(d.size() - 1, -1, -1):
		s += char(48 + d[i])
	return s.lpad(width, "0")


static func _round_at(xs: String, keep: int) -> String:
	## xs rounded to its first `keep` digits, half to even, the rest zeros, at the same width.
	if keep >= xs.length():
		return xs
	var head := xs.substr(0, keep)
	var next := xs.unicode_at(keep)
	var tie := next == 53 and xs.substr(keep + 1).lstrip("0") == ""
	if next > 53 or (next == 53 and (not tie or head.unicode_at(keep - 1) % 2 == 1)):
		var chars := head.to_utf8_buffer()
		var i := chars.size() - 1
		while i >= 0:
			if chars[i] == 57:
				chars[i] = 48
				i -= 1
			else:
				chars[i] += 1
				break
		head = chars.get_string_from_utf8()
		if i < 0:
			head = "1" + head
			return (head + "0".repeat(xs.length() - keep))   # one wider: compares above any bound
	return head + "0".repeat(xs.length() - keep)


static func _place(digits: String, frac: int) -> String:
	## Puts the point `frac` digits from the right; strips leading and trailing zeros.
	var s := digits.lpad(frac + 1, "0")
	var whole := s.substr(0, s.length() - frac).lstrip("0")
	var part := s.substr(s.length() - frac).rstrip("0") if frac > 0 else ""
	if whole == "":
		whole = "0"
	return whole if part == "" else whole + "." + part


static func _less(a: String, b: String) -> bool:
	## Bytewise order of the UTF-8 encodings.
	var x := a.to_utf8_buffer()
	var y := b.to_utf8_buffer()
	for n in mini(x.size(), y.size()):
		if x[n] != y[n]:
			return x[n] < y[n]
	return x.size() < y.size()
