"""The demo's pictures: from a sweep (sweep.py's OUT) the hero still and the sweep as video; from a
timelapse (timelapse.py's OUT) the gallery of knobs and the timelapse as video.

    python3 examples/demo/compose.py OUT

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


def knob_text(e):
    """A knob's one line, with its table where it has one: `[let] woods = 0.35`."""
    return (e["table"] + " " if e.get("table") else "") + e["line"]


def gallery(out, tl):
    """The base and each knob alone, 5 across, each lettered with its one line and its numbers."""
    tiles = [("the world as written", None, json.loads((out / "measures" / "base.json").read_text()),
              out / "frames" / "g_base.png")]
    tiles += [(g["says"], knob_text(g), g["measures"], out / "frames" / f"g_{g['knob']}.png") for g in tl["gallery"]]
    cols, tw, th, pad, gap = 5, 720, 405, 64, 20
    rows = (len(tiles) + cols - 1) // cols
    W = pad * 2 + cols * tw + (cols - 1) * gap
    top, cap = 230, 120
    H = top + rows * (th + cap + gap) + 110
    img = Image.new("RGB", (W, H), PAPER)
    d = ImageDraw.Draw(img)
    d.text((pad, 52), "One world, one line at a time.", font=font(SANS, 76, 1), fill=INK)
    d.text((pad, 148), f"The same base with each of its knobs turned alone: {len(tiles) - 1} one-line edits, each a "
           "real build, no house or tree placed by hand.", font=font(SANS, 34), fill=DIM)
    mono, big, small = font(MONO, 24), font(SANS, 30, 1), font(SANS, 22)
    for i, (says, text, m, file) in enumerate(tiles):
        x = pad + (i % cols) * (tw + gap)
        y = top + (i // cols) * (th + cap + gap)
        im = Image.open(file).convert("RGB").resize((tw, th), Image.LANCZOS)
        img.paste(im, (x, y))
        d.text((x, y + th + 12), says, font=big, fill=INK)
        if text:
            d.text((x, y + th + 54), text, font=mono, fill=GREEN)
        nums = (f"{m['buildings']} houses \u00b7 {thousands(m['trees'])} trees \u00b7 {m['warnings']} warnings")
        d.text((x, y + th + 88), nums, font=small, fill=DIM)
    godot = ".".join(tl["godot"].split(".")[:3])
    d.text((pad, H - 70), "Kenney's Modular Buildings (CC0) at a person's size and via's starter trees, drawn in Godot "
           f"{godot} from each build. Every number is a measure: examples/demo.", font=font(SANS, 24), fill=DIM)
    img.save(out / "gallery.png")
    return out / "gallery.png"


def timelapse(out, tl):
    """The states' orbit frames in order, each state's last `fade` frames blended into the next's
    first, lettered with the knobs turned so far."""
    folder = out / "lettered"
    folder.mkdir(exist_ok=True)
    mono, sans, small = font(MONO, 34), font(SANS, 34, 1), font(SANS, 26)
    states, fade, per = tl["states"], tl["fade"], tl["per"]
    k = 0
    for si, st in enumerate(states):
        nxt = states[si + 1] if si + 1 < len(states) else None
        for f in range(per):
            img = Image.open(out / "frames" / f"{st['name']}_{f:03d}.png").convert("RGB")
            if f < fade and si > 0:
                prev = states[si - 1]
                back = Image.open(out / "frames" / f"{prev['name']}_{per + f:03d}.png").convert("RGB")
                img = Image.blend(back, img, (f + 1) / (fade + 1))
            d = ImageDraw.Draw(img, "RGBA")
            # Through a blend the panel says the state the picture mostly is: the one before until
            # half way (D's review).
            shown = si - 1 if (si > 0 and f < fade // 2) else si
            st_shown = states[shown]
            turned = [s for s in states[1:shown + 1]]
            h = 70 + 44 * max(1, len(turned))
            m = st_shown["measures"]
            foot = (f"{m['buildings']} houses \u00b7 {thousands(m['trees'])} trees \u00b7 "
                    f"{m['warnings']} warnings \u00b7 built in {m['build_ms'] / 1000:.0f} s")
            wide = max([d.textlength(knob_text(s2), font=mono) for s2 in turned] + [d.textlength(foot, font=small), 520])
            d.rounded_rectangle((36, 36, 36 + 48 + wide, 36 + h + 56), 16, fill=(0, 0, 0, 140))
            d.text((60, 52), "via: the knobs turned so far", font=small, fill=(220, 220, 220))
            if not turned:
                d.text((60, 92), "the world as written", font=mono, fill=(255, 255, 255))
            for j, s2 in enumerate(turned):
                colour = (140, 230, 160) if j == len(turned) - 1 else (200, 200, 200)
                d.text((60, 92 + 44 * j), knob_text(s2), font=mono, fill=colour)
            d.text((60, 36 + h + 14), foot, font=small, fill=(230, 230, 230))
            if turned:
                d.text((img.width - 60 - d.textlength(st_shown["says"], font=sans), img.height - 84),
                       st_shown["says"], font=sans, fill=(255, 255, 255, 230))
            img.save(folder / f"{k:03d}.png")
            k += 1
    mp4, gif = out / "timelapse.mp4", out / "timelapse.gif"
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-framerate", "12", "-i", str(folder / "%03d.png"),
                    "-vf", "scale=1280:-2:flags=lanczos,fps=24,format=yuv420p", "-c:v", "libx264", "-crf", "26",
                    "-preset", "slow", "-movflags", "+faststart", str(mp4)], check=True)
    # The GIF is a preview for where video does not play: smaller and slower to load than the MP4.
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-framerate", "12", "-i", str(folder / "%03d.png"),
                    "-vf", "fps=8,scale=560:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=96[p];[b][p]paletteuse=dither=bayer:bayer_scale=4",
                    "-loop", "0", str(gif)], check=True)
    return mp4, gif


def main(argv):
    if len(argv) != 2:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    out = Path(argv[1]).resolve()
    if (out / "timelapse.json").exists():
        tl = json.loads((out / "timelapse.json").read_text())
        print("compose:", gallery(out, tl))
        print("compose:", *timelapse(out, tl))
        return 0
    sweep = json.loads((out / "sweep.json").read_text())
    print("compose:", hero(out, sweep))
    folder, n = frames(out, sweep)
    print("compose:", *video(out, folder, n))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
