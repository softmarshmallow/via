"""An export writing a file whose name is a Windows device name (build.md case 51)."""

import os

NAME = "aux.json"


def export(step):
    with open(os.path.join(step.out.folder, NAME), "w", encoding="utf-8") as f:
        f.write('{"format":"names-0"}\n')
