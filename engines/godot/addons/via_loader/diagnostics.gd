extends RefCounted
## The loader's diagnostics: records of spec/diagnostics.md 1, codes `load.*` of build.md's table,
## written as diagnostics.md 7's document. A loader reads no text, so every record has the `file`
## location form: `file` is a build path (build.md 3.4) and `data` locates inside it. When the sheet
## tool loads two builds, `data.build` says which ("before" or "after").

const CJson := preload("res://addons/via_loader/cjson.gd")
const SEVERITY_RANK := {"error": 0, "warning": 1, "info": 2}
const SEVERITY := {
	"load.format": "error",
	"load.bad_file": "error",
	"load.too_many_materials": "warning",
	"load.fields_not_drawn": "info",
	"load.bad_target": "warning",
}

var records: Array = []
## Added to every record's data when not empty: which build of a sheet this is.
var build_label := ""


func add(code: String, subject: String, file: String, message: String, hint: String, data := {}) -> void:
	var d: Dictionary = data.duplicate(true)
	if build_label != "":
		d["build"] = build_label
	records.append({
		"code": code, "data": d, "file": file, "hint": hint, "message": message,
		"severity": SEVERITY[code], "subject": subject,
	})


func errors() -> int:
	return records.filter(func(r): return r.severity == "error").size()


func status() -> int:
	return 1 if errors() > 0 else 0


func sorted_records() -> Array:
	## diagnostics.md 5's order, for records of the file form only: severity, file, code, subject,
	## then the record's canonical bytes.
	var out := records.duplicate()
	out.sort_custom(func(a, b):
		var ka := [SEVERITY_RANK[a.severity], a.file, a.code, a.subject, CJson.encode(a)]
		var kb := [SEVERITY_RANK[b.severity], b.file, b.code, b.subject, CJson.encode(b)]
		for n in ka.size():
			if ka[n] != kb[n]:
				return ka[n] < kb[n]
		return false)
	return out


func document() -> Dictionary:
	var counts := {"error": 0, "failed_on": 0, "info": 0, "warning": 0}
	for r in records:
		counts[r.severity] += 1
	return {"counts": counts, "diagnostics": sorted_records(), "format": "via-diagnostics-1",
		"lints": [], "status": status()}


func write(path: String) -> bool:
	var f := FileAccess.open(path, FileAccess.WRITE)
	if f == null:
		printerr("via loader: cannot write diagnostics to %s (%s)" % [path, error_string(FileAccess.get_open_error())])
		return false
	f.store_string(CJson.text(document()))
	f.close()
	return true


func print_all() -> void:
	## Without the `diagnostics` option they go to Godot's output, one line each, as the terminal
	## form of diagnostics.md 7 abbreviates them.
	for r in sorted_records():
		var line: String = "%s[%s] %s: %s\n  --> %s\n  hint: %s" % [r.severity, r.code, r.subject, r.message, r.file, r.hint]
		if r.severity == "error":
			printerr(line)
		else:
			print(line)
