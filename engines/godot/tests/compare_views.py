"""Compares the sheet's bookmark views of one build drawn two ways (spec/build.md 15.1, Streaming).

    python3 engines/godot/tests/compare_views.py WHOLE STREAMED [--most FRACTION] [--json FILE]
                                                [--jobs N]

WHOLE and STREAMED are two sheet folders of the same build (`sheet.sh --before BUILD`, without and
with `--stream RADIUS`). For every `before/<bookmark>.png` in both, the fraction of pixels that
differ: a pixel differs when any channel differs by more than DIFFERS (of 255). Prints one line per
view and the most; exits 0 when every view's fraction is at most --most (default MOST), 1
otherwise, 2 on bad arguments. Python's standard library only.

Why a view differs: when STREAMED was drawn with `--far-mask` (sheet.gd), each differing pixel is
put down to what the streamed view shows there, read from `before/far/<bookmark>.png` (the same
view with the far layers in mask colours, stream.gd): a far instance (the mask magenta), the far
ground (its skirts, water and the far meshes; the mask cyan), or what is loaded (the mask equals
the plain view there: an edge where the loaded and far parts meet, or a renderer's difference).
Each line then gives the three shares, the largest named as the reason, and the view is allowed
--most plus STAND_IN of the share of the view its far instances show (STAND_IN, below).
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
# Ours (M7, decided, open to the owner): with a far mask, a view may also differ in up to
# STAND_IN of the pixels its far instances show. A far instance stands for a model's outline by
# bands and its mean colour, not its facets: on atlas at 96 m (126 views, M7) the drawn models
# differed from their stand-ins in 20 to 45% of the pixels the stand-ins showed, so a view looking
# mostly past the load radius (vale.aerial: far instances 12.7% of it) cannot meet 1% by any
# stand-in short of the model itself.
STAND_IN = 0.5
REASON = {"instances": "far instances (stand-ins for trees and houses past the load radius)",
          "ground": "the far ground (coarse posts, its water and far meshes)",
          "loaded": "loaded content (seams at the load edge, or the renderer)"}


def near(pa, pb):
    return (abs(pa[0] - pb[0]) <= DIFFERS and abs(pa[1] - pb[1]) <= DIFFERS and abs(pa[2] - pb[2]) <= DIFFERS)


def differ(a, b, mask=None):
    """The fraction of pixels that differ, and with a mask the counts of those on far instances,
    the far ground and what is loaded ({"instances", "ground", "loaded"}), else None."""
    wa, ha, ra, bpa = png(a)
    wb, hb, rb, bpb = png(b)
    if (wa, ha) != (wb, hb):
        return 1.0, None
    rm = bpm = None
    if mask is not None and mask.exists():
        wm, hm, rm, bpm = png(mask)
        if (wm, hm) != (wa, ha):
            rm = None
    why = {"instances": 0, "ground": 0, "loaded": 0, "shown": 0} if rm is not None else None
    n = 0
    for y in range(ha):
        la, lb = ra[y], rb[y]
        lm = rm[y] if rm is not None else None
        for x in range(wa):
            pa, pb = x * bpa, x * bpb
            differs = (abs(la[pa] - lb[pb]) > DIFFERS or abs(la[pa + 1] - lb[pb + 1]) > DIFFERS
                       or abs(la[pa + 2] - lb[pb + 2]) > DIFFERS)
            n += differs
            if lm is None:
                continue
            pm = x * bpm
            m = (lm[pm], lm[pm + 1], lm[pm + 2])
            if near(m, (lb[pb], lb[pb + 1], lb[pb + 2])):
                why["loaded"] += differs
            elif m[0] > m[1]:
                why["instances"] += differs
                why["shown"] += 1   # a pixel a far instance shows
            else:
                why["ground"] += differs
    return n / (wa * ha), why


def PIXELS(path):
    w, h, _, _ = png(path)
    return w * h


def _one(job):
    whole, streamed, name = job
    return name, differ(whole / name, streamed / name, streamed / "far" / name)


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
    jobs = 1
    if "--jobs" in args:
        k = args.index("--jobs")
        jobs = int(args[k + 1])
        del args[k:k + 2]
    if len(args) != 2:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    whole, streamed = Path(args[0]) / "before", Path(args[1]) / "before"
    # the bookmark views only: map.png is the player map (build.md 15.1, The minimap), not a view
    names = sorted(p.name for p in whole.glob("*.png") if (streamed / p.name).exists() and p.name != "map.png")
    if not names:
        print("FAIL no view in both folders", file=sys.stderr)
        return 1
    work = [(whole, streamed, name) for name in names]
    if jobs > 1:
        import multiprocessing
        with multiprocessing.Pool(jobs) as pool:
            done = dict(pool.map(_one, work))
    else:
        done = dict(map(_one, work))
    res, whys, allowed = {}, {}, {}
    for name in names:
        v = name[:-4]
        res[v], why = done[name]
        whys[v] = why
        shown = why["shown"] / PIXELS(whole / name) if why else 0.0
        allowed[v] = most + STAND_IN * shown
        line = (f"{'ok  ' if res[v] <= allowed[v] else 'FAIL'} {v}: {res[v] * 100:.2f}% of pixels differ"
                + (f", at most {allowed[v] * 100:.2f}% (far instances show {shown * 100:.1f}%)" if why else ""))
        total = sum(c for k, c in why.items() if k != "shown") if why else 0
        if total:
            share = {k: c / total for k, c in why.items() if k != "shown"}
            top = max(share, key=share.get)
            line += (f" (far instances {share['instances'] * 100:.0f}%, far ground {share['ground'] * 100:.0f}%, "
                     f"loaded {share['loaded'] * 100:.0f}%: mostly {REASON[top]})")
        print(line)
    worst = max(res, key=res.get)
    over = sorted(v for v in res if res[v] > allowed[v])
    print(f"most: {worst} {res[worst] * 100:.2f}% (at most {most * 100:.2f}%, plus {STAND_IN:g} of what far "
          f"instances show when masked; a pixel differs by more than {DIFFERS} of 255); over: {len(over)}")
    if out:
        Path(out).write_text(json.dumps({"allowed": allowed, "differs": DIFFERS, "most": most, "stand_in": STAND_IN,
                                         "views": res, "why": whys}, indent=2, sort_keys=True))
    return 0 if not over else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
