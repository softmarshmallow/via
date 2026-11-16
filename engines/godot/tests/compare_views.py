"""Compares the sheet's bookmark views of one build drawn two ways (spec/build.md 15.1, Streaming).

    python3 engines/godot/tests/compare_views.py WHOLE STREAMED [--most FRACTION] [--json FILE]

WHOLE and STREAMED are two sheet folders of the same build (`sheet.sh --before BUILD`, without and
with `--stream RADIUS`). For every `before/<bookmark>.png` in both, the fraction of pixels that
differ: a pixel differs when any channel differs by more than DIFFERS (of 255). Prints one line per
view and the most; exits 0 when every view's fraction is at most --most (default MOST), 1
otherwise, 2 on bad arguments. Python's standard library only.
"""

import json
import runpy
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
png = runpy.run_path(str(HERE / "check_sheet.py"))["png"]

# Ours: 24 of 255 (about 9%) passes the renderer's own noise between two loads of one scene (MSAA
# edges, shadow-map texels) and catches a house, a roof or ground of another colour.
DIFFERS = 24
# Ours: a streamed view may differ from the whole one in at most 1% of its pixels: a far house is a
# box of its bounds in one colour, not its model (the far instances), and the far ground is coarse.
MOST = 0.01


def differ(a, b):
    wa, ha, ra, bpa = png(a)
    wb, hb, rb, bpb = png(b)
    if (wa, ha) != (wb, hb):
        return 1.0
    n = 0
    for y in range(ha):
        la, lb = ra[y], rb[y]
        for x in range(wa):
            pa, pb = x * bpa, x * bpb
            if (abs(la[pa] - lb[pb]) > DIFFERS or abs(la[pa + 1] - lb[pb + 1]) > DIFFERS
                    or abs(la[pa + 2] - lb[pb + 2]) > DIFFERS):
                n += 1
    return n / (wa * ha)


def main(argv):
    args = list(argv)
    most, out = MOST, None
    if "--most" in args:
        k = args.index("--most")
        most = float(args[k + 1])
        del args[k:k + 2]
    if "--json" in args:
        k = args.index("--json")
        out = args[k + 1]
        del args[k:k + 2]
    if len(args) != 2:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    whole, streamed = Path(args[0]) / "before", Path(args[1]) / "before"
    names = sorted(p.name for p in whole.glob("*.png") if (streamed / p.name).exists())
    if not names:
        print("FAIL no view in both folders", file=sys.stderr)
        return 1
    res = {}
    for name in names:
        res[name[:-4]] = differ(whole / name, streamed / name)
        print(f"{'ok  ' if res[name[:-4]] <= most else 'FAIL'} {name[:-4]}: {res[name[:-4]] * 100:.2f}% of pixels differ")
    worst = max(res, key=res.get)
    print(f"most: {worst} {res[worst] * 100:.2f}% (at most {most * 100:.2f}%; a pixel differs by more than {DIFFERS} of 255)")
    if out:
        Path(out).write_text(json.dumps({"differs": DIFFERS, "most": most, "views": res}, indent=2, sort_keys=True))
    return 0 if res[worst] <= most else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
