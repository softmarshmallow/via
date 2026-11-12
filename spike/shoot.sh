#!/bin/zsh
# Render every bookmark of a compiled world.  shoot.sh [BUILD_NAME] [--only=a,b]
# It always asks the viewer for --shots; passing it again is harmless.
# The window is parked as an unfocused sliver; the picture is drawn off screen at 1920x1080.
SPIKE="${0:A:h}"
NAME="${1:-vale}"
shift 2>/dev/null
LOG=$(mktemp -t worldc-shot)
RUN="worldc-run-$$"
( sleep 240; pkill -f -- "--$RUN" ) &
WATCH=$!
caffeinate -dimsu open -g -n -W --stdout "$LOG" --stderr "$LOG" -a /Applications/Godot.app --args --audio-driver Dummy \
  --path "$SPIKE/viewer" -- "--build=$SPIKE/build/$NAME" "--out=$SPIKE/out/$NAME" --shots "--$RUN" "$@"
kill $WATCH 2>/dev/null
grep -iv "godot engine\|metal \|vulkan" "$LOG" | grep -v "^ *$" | head -${LINES_MAX:-40}
rm -f "$LOG"
