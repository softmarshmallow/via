extends RefCounted
## Lays rendered bookmark views out on one sheet (spec/build.md 15.1, the sheet tool): a header
## band 48 pixels high, then one row per bookmark: a label band 24 pixels high and the before and
## after views side by side, each scaled to 640 by 360. 1280 wide with an after build, else 640;
## `48 + 384 * rows` high. A view missing from one build is a grey cell (#808080) marked "absent".
## Drawn by Godot's own 2D renderer in an off-screen viewport, so text needs no image library.

const CELL := Vector2i(640, 360)
const HEADER := 48
const LABEL := 24
const ABSENT := Color8(0x80, 0x80, 0x80)
const BAND := Color8(0x20, 0x22, 0x26)
const INK := Color8(0xee, 0xee, 0xee)


static func size_for(rows: int, two: bool) -> Vector2i:
	return Vector2i(CELL.x * (2 if two else 1), HEADER + (LABEL + CELL.y) * rows)


## rows: [{ "label": String, "before": Image or null, "after": Image or null }] in name order.
static func compose(host: Node, header: String, rows: Array, two: bool) -> Image:
	var size := size_for(rows.size(), two)
	var vp := SubViewport.new()
	vp.size = size
	vp.disable_3d = true
	vp.transparent_bg = false
	vp.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	host.add_child(vp)
	var bg := ColorRect.new()
	bg.color = BAND
	bg.size = Vector2(size)
	vp.add_child(bg)
	_label(vp, header, Vector2(12, 0), Vector2(size.x - 24, HEADER), 20)
	var y := HEADER
	for row in rows:
		_label(vp, row.label, Vector2(8, y), Vector2(size.x - 16, LABEL), 14)
		y += LABEL
		var sides := ["before", "after"] if two else ["before"]
		for n in sides.size():
			var at := Vector2(n * CELL.x, y)
			var img = row.get(sides[n])
			if img is Image:
				var tex := TextureRect.new()
				tex.texture = ImageTexture.create_from_image(img)
				tex.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
				tex.stretch_mode = TextureRect.STRETCH_SCALE
				tex.position = at
				tex.size = Vector2(CELL)
				vp.add_child(tex)
			else:
				var grey := ColorRect.new()
				grey.color = ABSENT
				grey.position = at
				grey.size = Vector2(CELL)
				vp.add_child(grey)
				var l := _label(vp, "absent", at, Vector2(CELL), 24)
				l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		y += CELL.y
	for f in 3:
		await RenderingServer.frame_post_draw
	var out := vp.get_texture().get_image()
	vp.queue_free()
	out.convert(Image.FORMAT_RGB8)
	return out


static func _label(parent: Node, text: String, at: Vector2, box: Vector2, px: int) -> Label:
	var l := Label.new()
	l.text = text
	l.position = at
	l.size = box
	l.clip_text = true
	l.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	l.add_theme_font_size_override("font_size", px)
	l.add_theme_color_override("font_color", INK)
	parent.add_child(l)
	return l
