#!/bin/sh
# The Godot loader suite: engines/godot/tests/run.sh
# Runs on macOS with a display (Godot's --headless draws nothing and keeps no MultiMesh data).
# GODOT names the binary; the default is the pinned release's macOS install. Writes only under
# engines/godot/tests/out/, or under GODOT_TESTS_OUT when set (two runs at once each need their
# own: the suite removes its folder first). Exit 0 when every part passes.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/.." && pwd)
GODOT=${GODOT:-/Applications/Godot.app/Contents/MacOS/Godot}
OUT="${GODOT_TESTS_OUT:-$HERE/out}"
fail=0
step() { echo "== $1"; }
godot() { "$GODOT" --path "$ROOT" --rendering-method gl_compatibility --audio-driver Dummy "$@" 2>&1 \
  | grep -v "^Godot Engine\|^OpenGL API\|^$"; }

step "fixture builds match their generator"
python3 "$HERE/make_fixture.py" --check || fail=1

step "loader"
quiet() { # show only what did not pass; keep the command's status
  out=$("$@"); s=$?; printf '%s\n' "$out" | grep -v "^ok"; return $s; }
r=$(godot --script res://tests/test_loader.gd); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "walker and hand-edit marks"
r=$(godot --script res://tests/test_walk.gd); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "water in the frame at at_ford"
godot --script res://tests/test_water.gd || fail=1

step "exposure: known colours under the sheet's rig, not clipped, hue kept"
r=$(godot --script res://tests/test_exposure.gd); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "sheet: before and after"
rm -rf "$OUT" && mkdir -p "$OUT"
"$ROOT/sheet.sh" --before "$HERE/fixture_build" --after "$HERE/fixture_build+downstream" --out "$OUT/pair" \
  --option water=tag:water_level --option paths=kind:lane >/dev/null 2>"$OUT/pair.err"
s=$?; [ $s -eq 0 ] || { echo "FAIL sheet exit $s"; tail -20 "$OUT/pair.err"; fail=1; }
quiet python3 "$HERE/check_sheet.py" "$OUT/pair" 5 --before "$HERE/fixture_build" --after "$HERE/fixture_build+downstream" || fail=1
python3 - "$OUT/pair/diagnostics.json" <<'EOF' || fail=1
import json, sys
d = json.load(open(sys.argv[1]))
codes = [(r["code"], r["subject"], r["data"].get("build")) for r in d["diagnostics"]]
want = ("load.bad_target", "feature.broken.toward", "after")
ok = want in codes and d["status"] == 0
print(("ok   " if ok else "FAIL ") + "sheet diagnostics: the after build's bad bookmark is load.bad_target", codes)
sys.exit(0 if ok else 1)
EOF
python3 - "$OUT/pair/sheet.json" <<'EOF' || fail=1
import json, sys
marks = {b["name"]: b for b in json.load(open(sys.argv[1]))["bookmarks"]}
e = marks["at_ford"]["eye"]
d = marks["downstream_view"]["eye"]
t = marks["at_ford"]["target"]
dt = marks["downstream_view"]["target"]
ok = (e["before"] != e["after"] and d["before"] is None and d["after"] is not None
      and t["before"]["at"] != t["after"]["at"] and dt["before"] is None and dt["after"] is not None)
print(("ok   " if ok else "FAIL ") + "sheet.json: at_ford's eye and target moved, each given per build; a bookmark in one build has one", e, t, d, dt)
sys.exit(0 if ok else 1)
EOF

step "sheet: pages of two bookmarks (5 bookmarks, 3 pages)"
"$ROOT/sheet.sh" --before "$HERE/fixture_build" --after "$HERE/fixture_build+downstream" --out "$OUT/paged" \
  --option water=tag:water_level --option paths=kind:lane --page 2 >/dev/null 2>&1 || { echo "FAIL sheet exit"; fail=1; }
quiet python3 "$HERE/check_sheet.py" "$OUT/paged" 5 --page 2 --before "$HERE/fixture_build" --after "$HERE/fixture_build+downstream" || fail=1

step "sheet: one build, rewritten in place"
"$ROOT/sheet.sh" --before "$HERE/fixture_build" --out "$OUT/one" >/dev/null 2>&1 || { echo "FAIL sheet exit"; fail=1; }
"$ROOT/sheet.sh" --before "$HERE/fixture_build" --out "$OUT/one" >/dev/null 2>&1 || { echo "FAIL second run over its own folder"; fail=1; }
quiet python3 "$HERE/check_sheet.py" "$OUT/one" 4 --one --before "$HERE/fixture_build" || fail=1

step "sheet: bad arguments exit 2 and write nothing"
mkdir -p "$OUT/foreign" && echo x > "$OUT/foreign/keep.txt"
for args in "--before $HERE/fixture_build --out $OUT/foreign" \
            "--before $HERE/fixture_build --out $OUT/bad --option colour=red" \
            "--before $HERE/fixture_build --out $OUT/bad --option features=yes" \
            "--before $HERE/fixture_build --out $OUT/bad --option diagnostics=x" \
            "--before $HERE/fixture_build --out $OUT/bad --stream 0" \
            "--before $HERE/fixture_build --out $OUT/bad --frames x" \
            "--before $HERE/fixture_build --out $OUT/bad --page 0" \
            "--before $HERE/fixture_build --out $OUT/bad --far-mask" \
            "--out $OUT/bad"; do
  # shellcheck disable=SC2086
  "$ROOT/sheet.sh" $args >/dev/null 2>&1
  s=$?
  if [ $s -ne 2 ] || [ -e "$OUT/bad" ] || [ ! -f "$OUT/foreign/keep.txt" ]; then echo "FAIL exit $s for: $args"; fail=1; else echo "ok   exit 2: $args"; fi
done

step "kind surfaces: cut terrain, floor meshes, the walker on a floor (headless)"
python3 "$HERE/make_floor_fixture.py" "$OUT/floor_build" || fail=1
r=$(godot --headless --script res://tests/test_floor.gd -- --build "$OUT/floor_build"); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "far layer: posts 8 m apart, skirts, far water, instances by bands, far meshes (headless)"
r=$(godot --headless --script res://tests/test_far.gd -- --build "$HERE/fixture_build" --water tag:water_level --meshes "$OUT/floor_build"); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "streaming: content once, a long walk loading and unloading chunks, load-unload-load (headless)"
# fixture_build is 96 m by 80 m in four 64 m chunks: a 5 m load radius (unloading past 69 m) makes
# the walk unload some, and is past the 4 m guard, so a walk loading one chunk a frame never stalls
r=$(godot --headless --fixed-fps 60 --script res://tests/test_stream.gd -- --build "$HERE/fixture_build" --radius 5); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1
r=$(godot --headless --fixed-fps 60 --script res://tests/test_stream.gd -- --build "$HERE/fixture_build" --radius 5 --frames); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "streamed walks against whole walks, wading: reproducible, same outcome and blocker, stop within two frames' pace (headless)"
r=$(godot --headless --script res://tests/test_stream_walks.gd -- --build "$HERE/fixture_build" --radius 5 --water tag:water_level); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "reload after a rebuild: kept and made again by chunk digest (headless; the ford's move changes 3 of the 4 chunks' digests)"
r=$(godot --headless --script res://tests/test_reload.gd -- --before "$HERE/fixture_build" --after "$HERE/fixture_build+downstream" --radius 5 --mixed); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "minimap: station's player map read, turned with the walker, per floor (headless; fixture_map is station's export)"
r=$(godot --headless --script res://tests/test_minimap.gd -- --build "$HERE/fixture_map"); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "sheet: the player map as before/map.png and the corner map on each walk's eye view"
# fixture_build with fixture_map's export (station's map) beside it, named in its manifest's exports
python3 - "$HERE/fixture_build" "$HERE/fixture_map" "$OUT/map_build" <<'EOF' || fail=1
import json, os, shutil, sys
from pathlib import Path
src, mp, dst = map(Path, sys.argv[1:])
shutil.copytree(src, dst)
shutil.copytree(mp / "export", dst / "export")
m = json.loads((dst / "manifest.json").read_text())
m["exports"] = json.loads((mp / "manifest.json").read_text())["exports"]
m["locations"]["pack_root"] = os.path.relpath((src / m["locations"]["pack_root"]).resolve(), dst.resolve())
(dst / "manifest.json").write_text(json.dumps(m, sort_keys=True, separators=(",", ":")) + "\n")
EOF
"$ROOT/sheet.sh" --before "$OUT/map_build" --out "$OUT/map" >/dev/null 2>&1 || { echo "FAIL sheet exit"; fail=1; }
quiet python3 "$HERE/check_sheet.py" "$OUT/map" 4 --one --map || fail=1

step "sheet: the surface each bookmark stands on"
"$ROOT/sheet.sh" --before "$OUT/floor_build" --out "$OUT/floor" >/dev/null 2>&1 || { echo "FAIL sheet exit"; fail=1; }
quiet python3 "$HERE/check_sheet.py" "$OUT/floor" 2 --one --before "$OUT/floor_build" || fail=1
python3 - "$OUT/floor/sheet.json" <<'EOF' || fail=1
import json, sys
walks = {w["name"]: w["before"] for w in json.load(open(sys.argv[1]))["walks"]}
on = {n: w["on"] for n, w in walks.items()}
ok = on == {"below_floor": {"from": None, "stop": None}, "on_floor": {"from": "hall.floor", "stop": "hall.floor"}}
print(("ok   " if ok else "FAIL ") + "sheet.json: on_floor stands on hall.floor, below_floor on nothing (the cut)", on)
sys.exit(0 if ok else 1)
EOF

[ $fail -eq 0 ] && echo "PASS: Godot loader suite" || echo "FAIL: Godot loader suite"
exit $fail
