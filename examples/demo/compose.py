"""The demo's pictures from a sweep (sweep.py's OUT): the hero still and the sweep as video.

    python3 examples/demo/compose.py OUT [--hero-to FILE.png] [--video-to FILE.mp4] [--gif-to FILE.gif]

- **The hero** (`OUT/hero.png`): the base and one step side by side, the one line that differs
  between them, and the numbers the measures read from the two builds. Every number is a measure
  of examples/demo/measure.py, as `OUT/measures/` holds it.
- **The sweep** (`OUT/sweep.mp4`, `OUT/sweep.gif`): each step's frame in the order of the line's
  number, out and back, lettered with the line and its build's numbers.

Needs PIL (Pillow) and ffmpeg. Ours; the layout is chosen by looking.
"""

import json
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SANS = "/System/Library/Fonts/Helvetica.ttc"
MONO = "/System/Library/Fonts/Menlo.ttc"
INK = (24, 27, 33)
PAPER = (246, 244, 239)
DIM = (110, 112, 118)
RED, GREEN = (178, 52, 46), (34, 128, 62)


def font(path, size, index=0):
    try:
        return ImageFont.truetype(path, size, index=index)
    except OSError:
        return ImageFont.load_default()


def thousands(n):
    return f"{n:,}"


def numbers(m, b):
    """The hero's lines, from one step's measures `m` against the base's `b`."""
    return [
        (f"{b['buildings']} houses to {m['buildings']}",
         f"every one re-laid from the kit, every door on its street ({m['doors_on_street']} of {m['doors']})"),
        (f"{thousands(m['trees_added'])} trees grew, {thousands(m['trees_removed'])} gave way",
         f"{thousands(m['trees_kept'])} stayed where they stood; none on a house's ground"),
        (f"{m['overlap_m2']:g} m² overlap", f"{m['errors']} errors, {m['warnings']} warnings"),
        (f"{m['nodes_ran']:,} of {m['nodes']:,} build nodes ran again", f"in {m['build_ms'] / 1000:.0f} s on a laptop"),
        (f"{b['hand_placed']} placed by hand", "no house, tree or wall piece is written in the world"),
    ]


def hero(out, sweep):
    before = Image.open(out / "frames" / "hero_before.png").convert("RGB")
    after = Image.open(out / "frames" / "hero_after.png").convert("RGB")
    w, h = 1400, int(1400 * before.height / before.width)
    before, after = before.resize((w, h), Image.LANCZOS), after.resize((w, h), Image.LANCZOS)
    pad, gap = 64, 24
    W = pad * 2 + w * 2 + gap
    top = 210
    H = top + h + 500
    img = Image.new("RGB", (W, H), PAPER)
    d = ImageDraw.Draw(img)
    d.text((pad, 52), "One line changes. The whole town follows.", font=font(SANS, 76, 1), fill=INK)
    d.text((pad, 146), "via builds a world from text: decide what matters, and the rest settles around it.",
           font=font(SANS, 34), fill=DIM)
    img.paste(before, (pad, top))
    img.paste(after, (pad + w + gap, top))
    lab = font(SANS, 30, 1)
    for x, word in ((pad, "before"), (pad + w + gap, "after")):
        d.rounded_rectangle((x + 20, top + 20, x + 20 + d.textlength(word, font=lab) + 32, top + 70), 10, fill=(0, 0, 0))
        d.text((x + 36, top + 26), word, font=lab, fill=(255, 255, 255))
    y0 = top + h + 44
    base = next(s for s in sweep["steps"] if s["name"] == "base")
    step = next(s for s in sweep["steps"] if s["y"] == sweep["hero"])
    mono = font(MONO, 34)
    d.text((pad, y0), "world.toml, the only edit", font=font(SANS, 26, 1), fill=DIM)
    d.rounded_rectangle((pad, y0 + 42, pad + 920, y0 + 182), 12, fill=(255, 255, 255), outline=(220, 218, 212))
    d.text((pad + 28, y0 + 58), "[feature.high_street]", font=mono, fill=INK)
    d.text((pad + 28, y0 + 98), "- " + base["line"], font=mono, fill=RED)
    d.text((pad + 28, y0 + 138), "+ " + step["line"], font=mono, fill=GREEN)
    x1 = pad + 1000
    big, small = font(SANS, 36, 1), font(SANS, 26)
    for i, (a, b) in enumerate(numbers(step["measures"], base["measures"])):
        yy = y0 + i * 66
        d.text((x1, yy), a, font=big, fill=INK)
        d.text((x1 + d.textlength(a, font=big) + 18, yy + 9), b, font=small, fill=DIM)
    godot = ".".join(sweep["godot"].split(".")[:3])
    foot = [f"Rebuilt and drawn from the text: the high street moved {abs(step['y'] - base['y'])} m north; its rows of houses follow it "
            "and re-fill, the woods give way and grow back, the ground is levelled under every house.",
            f"Kenney's Modular Buildings (CC0) at a person's size and via's starter trees, drawn in Godot {godot} from the build. "
            "Every number is a measure with its protocol: examples/demo/expect.toml."]
    for i, line in enumerate(foot):
        d.text((pad, H - 92 + i * 34), line, font=font(SANS, 24), fill=DIM)
    img.save(out / "hero.png")
    return out / "hero.png"


def frames(out, sweep):
    """Each step lettered, out and back, numbered for ffmpeg."""
    folder = out / "lettered"
    folder.mkdir(exist_ok=True)
    steps = sorted(sweep["steps"], key=lambda s: s["y"])
    order = steps + steps[-2:0:-1]
    mono, sans, small = font(MONO, 40), font(SANS, 38, 1), font(SANS, 28)
    for k, s in enumerate(order):
        img = Image.open(out / "frames" / f"{s['name']}.png").convert("RGB")
        d = ImageDraw.Draw(img, "RGBA")
        d.rounded_rectangle((40, 40, 1180, 232), 18, fill=(0, 0, 0, 150))
        d.text((70, 62), "[feature.high_street]", font=mono, fill=(255, 255, 255))
        d.text((70, 112), s["line"], font=mono, fill=(140, 230, 160))
        m = s["measures"]
        d.text((70, 174), f"{m['buildings']} houses · {thousands(m['trees'])} trees · {m['warnings']} warnings · "
               f"built in {m['build_ms'] / 1000:.0f} s", font=small, fill=(230, 230, 230))
        d.text((img.width - 560, img.height - 80), "via: one line, the town follows", font=sans, fill=(255, 255, 255, 220))
        img.save(folder / f"{k:03d}.png")
    return folder, len(order)


def video(out, folder, n):
    mp4, gif = out / "sweep.mp4", out / "sweep.gif"
    rate = "1.25"
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-framerate", rate, "-i", str(folder / "%03d.png"),
                    "-vf", "fps=25,format=yuv420p", "-c:v", "libx264", "-crf", "20", "-movflags", "+faststart",
                    str(mp4)], check=True)
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-framerate", rate, "-i", str(folder / "%03d.png"),
                    "-vf", "scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=192[p];[b][p]paletteuse=dither=sierra2_4a",
                    "-loop", "0", str(gif)], check=True)
    return mp4, gif


def main(argv):
    if len(argv) != 2:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    out = Path(argv[1]).resolve()
    sweep = json.loads((out / "sweep.json").read_text())
    print("compose:", hero(out, sweep))
    folder, n = frames(out, sweep)
    print("compose:", *video(out, folder, n))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
