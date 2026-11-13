@tool
extends EditorPlugin
## Project > Tools > "Load via build...": pick a build directory; the loader makes its node tree and
## the editor saves it as `res://<world>.tscn` (compile-time export). The same loader code runs in
## the sheet tool at start (build.md 15.1). Diagnostics go to the editor's output.

const Loader := preload("res://addons/via_loader/loader.gd")
const MENU := "Load via build..."

var _dialog: EditorFileDialog


func _enter_tree() -> void:
	add_tool_menu_item(MENU, _pick)


func _exit_tree() -> void:
	remove_tool_menu_item(MENU)
	if _dialog != null:
		_dialog.queue_free()


func _pick() -> void:
	if _dialog == null:
		_dialog = EditorFileDialog.new()
		_dialog.file_mode = EditorFileDialog.FILE_MODE_OPEN_DIR
		_dialog.access = EditorFileDialog.ACCESS_FILESYSTEM
		_dialog.title = "Choose a via build directory"
		_dialog.dir_selected.connect(_load)
		EditorInterface.get_base_control().add_child(_dialog)
	_dialog.popup_file_dialog()


func _load(dir: String) -> void:
	var loader := Loader.new()
	var root := loader.load_build(dir, {"water": "tag:water_level", "features": true})
	if root == null:
		push_error("via loader: nothing loaded from %s %s" % [dir, loader.option_error])
		return
	_own(root, root)
	var scene := PackedScene.new()
	var err := scene.pack(root)
	var path := "res://%s.tscn" % root.name
	if err == OK:
		err = ResourceSaver.save(scene, path)
	root.free()
	if err != OK:
		push_error("via loader: cannot save %s (%s)" % [path, error_string(err)])
		return
	EditorInterface.get_resource_filesystem().scan()
	EditorInterface.open_scene_from_path(path)


func _own(node: Node, owner: Node) -> void:
	for child in node.get_children():
		child.owner = owner
		_own(child, owner)
