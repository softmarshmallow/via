#!/bin/sh
# The Godot loader suite: engines/godot/tests/run.sh
# Runs on macOS with a display (Godot's --headless draws nothing and keeps no MultiMesh data).
# GODOT names the binary; the default is the pinned release's macOS install. Writes only under
# engines/godot/tests/out/. Exit 0 when every part passes.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/.." && pwd)
GODOT=${GODOT:-/Applications/Godot.app/Contents/MacOS/Godot}
OUT="$HERE/out"
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

step "water in the frame at at_ford"
godot --script res://tests/test_water.gd || fail=1

step "exposure: known colours under the sheet's rig, not clipped, hue kept"
r=$(godot --script res://tests/test_exposure.gd); printf '%s\n' "$r" | grep -v "^ok"
[ "$(printf '%s\n' "$r" | tail -1)" = "PASS: 0 check(s) failed" ] || fail=1

step "sheet: before and after"
rm -rf "$OUT"
"$ROOT/sheet.sh" --before "$HERE/fixture_build" --after "$HERE/fixture_build+downstream" --out "$OUT/pair" \
  --option water=tag:water_level --option paths=kind:lane >/dev/null 2>&1
s=$?; [ $s -eq 0 ] || { echo "FAIL sheet exit $s"; fail=1; }
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
            "--out $OUT/bad"; do
  # shellcheck disable=SC2086
  "$ROOT/sheet.sh" $args >/dev/null 2>&1
  s=$?
  if [ $s -ne 2 ] || [ -e "$OUT/bad" ] || [ ! -f "$OUT/foreign/keep.txt" ]; then echo "FAIL exit $s for: $args"; fail=1; else echo "ok   exit 2: $args"; fi
done

[ $fail -eq 0 ] && echo "PASS: Godot loader suite" || echo "FAIL: Godot loader suite"
exit $fail
