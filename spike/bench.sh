#!/bin/zsh
# Frame rate at every bookmark, in the walk-mode window.  bench.sh [BUILD_NAME]
SPIKE="${0:A:h}"
NAME="${1:-vale_local}"
LOG=$(mktemp -t worldc-bench)
RUN="worldc-bench-$$"
( sleep 300; pkill -f -- "--$RUN" ) &
WATCH=$!
caffeinate -dimsu open -n -W --stdout "$LOG" --stderr "$LOG" -a /Applications/Godot.app --args --audio-driver Dummy \
  --path "$SPIKE/viewer" -- "--build=$SPIKE/build/$NAME" "--out=$SPIKE/out/$NAME" --bench "--$RUN"
kill $WATCH 2>/dev/null
grep 'BENCH\|ERROR' "$LOG"
rm -f "$LOG"
