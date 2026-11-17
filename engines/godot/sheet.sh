#!/bin/sh
# Runs the sheet tool (spec/build.md 15.1) with a watchdog, so an unattended run never waits on a
# renderer that does not quit. Arguments are the tool's own:
#   engines/godot/sheet.sh --before DIR [--after DIR] --out DIR [--pack DIR] [--option NAME=VALUE ...]
# GODOT names the binary; the default is the pinned release's macOS install. Exit status is the
# tool's; when the watchdog ends a run that had already written <out>/diagnostics.json, the status
# recorded there is used (Godot can stall on quit after its work is done).
HERE=$(cd "$(dirname "$0")" && pwd)
GODOT=${GODOT:-/Applications/Godot.app/Contents/MacOS/Godot}
LIMIT=${SHEET_TIMEOUT:-1800}
want=$(cat "$HERE/GODOT_VERSION")
have=$("$GODOT" --version 2>/dev/null)
case "$have" in
  "$want"*) ;;
  *) echo "sheet.sh: Godot $have, but engines/godot pins $want (set GODOT)" >&2; exit 2 ;;
esac
out=""
prev=""
for a in "$@"; do
  [ "$prev" = "--out" ] && out="$a"
  prev="$a"
done
case "$out" in /*) ;; ?*) out="$PWD/$out" ;; esac
start=$(date +%s)
"$GODOT" --path "$HERE" --rendering-method gl_compatibility --audio-driver Dummy res://sheet.tscn -- "$@" &
pid=$!
# The watchdog's output is closed, so a pipe on this script's output ends when Godot does.
( sleep "$LIMIT"; kill -9 "$pid" ) >/dev/null 2>&1 </dev/null &
dog=$!
wait "$pid"
status=$?
pkill -P "$dog" 2>/dev/null
kill "$dog" 2>/dev/null
if [ "$status" -ge 128 ] && [ -n "$out" ] && [ -f "$out/diagnostics.json" ] \
   && [ "$(stat -f %m "$out/diagnostics.json" 2>/dev/null || stat -c %Y "$out/diagnostics.json")" -ge "$start" ]; then
  echo "sheet.sh: Godot stalled after writing $out; using its recorded status" >&2
  status=$(sed -n 's/.*"status":\([0-9]*\).*/\1/p' "$out/diagnostics.json" | tail -1)
fi
exit "$status"
