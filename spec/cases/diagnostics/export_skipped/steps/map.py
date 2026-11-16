"""An export that would write one file; the build's error keeps it from running."""

import os


def export(step):
    with open(os.path.join(step.out.folder, "map.json"), "w", encoding="utf-8") as f:
        f.write("{}\n")
