"""An export step that writes two files and one diagnostic (build.md case 47)."""

import os


def export(step):
    with open(os.path.join(step.out.folder, "map.json"), "w", encoding="utf-8") as f:
        f.write('{"format":"via-map-0"}\n')
    os.makedirs(os.path.join(step.out.folder, "floors"), exist_ok=True)
    with open(os.path.join(step.out.folder, "floors", "0.json"), "w", encoding="utf-8") as f:
        f.write('{"index":0}\n')
    step.info("step.map.written", "step.map", "two files", "none", explain="What was written.")
