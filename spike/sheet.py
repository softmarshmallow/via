"""Contact sheet of a build's bookmark renders.  sheet.py NAME [NAME2 ...]  ->  out/NAME/_sheet.png"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw
root = Path(__file__).parent / "out"
names = sys.argv[1:]
shots = sorted(p.name for p in (root / names[0]).glob("bm_*.png"))
w, h = 800, 450
sheet = Image.new("RGB", (w * len(names) if len(names) > 1 else w * 2, h * (len(shots) if len(names) > 1 else (len(shots) + 1) // 2)))
for k, shot in enumerate(shots):
    for c, name in enumerate(names):
        p = root / name / shot
        if not p.exists():
            continue
        im = Image.open(p).convert("RGB").resize((w, h), Image.LANCZOS)
        ImageDraw.Draw(im).text((8, 6), f"{name} / {shot[:-4]}", fill=(255, 255, 255), stroke_width=2, stroke_fill=(0, 0, 0))
        pos = (c * w, k * h) if len(names) > 1 else ((k % 2) * w, (k // 2) * h)
        sheet.paste(im, pos)
out = root / names[0] / ("_sheet.png" if len(names) == 1 else f"_vs_{'_'.join(names[1:])}.png")
sheet.save(out)
print(out)
