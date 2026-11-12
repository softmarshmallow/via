#!/bin/zsh
# Walk a compiled world.  explore.sh [BUILD_NAME]   (default: vale_local if present, else vale)
SPIKE="${0:A:h}"
NAME="${1:-$([[ -d "$SPIKE/build/vale_local" ]] && echo vale_local || echo vale)}"
exec godot --path "$SPIKE/viewer" -- "--build=$SPIKE/build/$NAME"
