"""An export that writes the time, so a rerun gives other bytes (build.md case 50)."""

import os
import time


def export(step):
    with open(os.path.join(step.out.folder, "stamp.json"), "w", encoding="utf-8") as f:
        f.write('{"ns":%d}\n' % time.time_ns())
