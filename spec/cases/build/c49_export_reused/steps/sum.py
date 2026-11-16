"""A worker export that reads height alone and writes its sum (build.md case 49)."""

import os


def export(step):
    (h,) = step.fields(["height"])
    with open(os.path.join(step.out.folder, "sum.json"), "w", encoding="utf-8") as f:
        f.write('{"sum":%r}\n' % round(float(h.sum()), 3))
