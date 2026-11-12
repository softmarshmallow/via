"""Tools around the emitted map: measure a restyled map against the data, draw icons and labels on
it, and list which tiles a level change invalidates.

    maptool.py measure MAPDIR STYLED.png
    maptool.py compose MAPDIR STYLED.png OUT.png
    maptool.py changed OLD_MAPDIR NEW_MAPDIR
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import binary_erosion, distance_transform_edt


def load(mapdir):
    meta = json.loads((Path(mapdir) / "map.json").read_text())
    cls = np.asarray(Image.open(Path(mapdir) / "class_index.png"))
    return meta, cls


def styled_to_grid(path, meta):
    t = meta["transform"]
    img = Image.open(path).convert("RGB")
    native = img.size
    return np.asarray(img.resize((t["width_px"], t["height_px"]), Image.LANCZOS), dtype=np.float32), native


def edge(mask):
    return mask & ~binary_erosion(mask)


def measure(mapdir, styled):
    meta, cls = load(mapdir)
    t = meta["transform"]
    rgb, native = styled_to_grid(styled, meta)
    want = t["width_px"] / t["height_px"]
    got = native[0] / native[1]
    print(f"picture: {native[0]}x{native[1]} (aspect {got:.3f}); the map is {t['width_px']}x{t['height_px']} (aspect {want:.3f}); "
          f"stretch to fit {abs(got / want - 1) * 100:.1f}%")
    per_m = t["px_per_m"]
    truth_water = cls <= 1
    # learn what the painter used for water and for land from where the data says each is, then
    # classify every pixel by the nearer of the two. The data teaches colour, not position.
    core_w = binary_erosion(truth_water, iterations=12)
    core_l = binary_erosion(~truth_water, iterations=12)
    mw, ml = rgb[core_w].mean(axis=0), rgb[core_l].mean(axis=0)
    pred_water = np.linalg.norm(rgb - mw, axis=2) < np.linalg.norm(rgb - ml, axis=2)
    iou = (pred_water & truth_water).sum() / max((pred_water | truth_water).sum(), 1)
    te, pe = edge(truth_water), edge(pred_water)
    d_to_pred = distance_transform_edt(~pe)[te] / per_m
    d_to_truth = distance_transform_edt(~te)[pe] / per_m
    print(f"water: overlap (IoU) {iou:.3f}")
    print(f"shore, data to picture: mean {d_to_pred.mean():.1f} m, 95th percentile {np.percentile(d_to_pred, 95):.1f} m")
    print(f"shore, picture to data: mean {d_to_truth.mean():.1f} m, 95th percentile {np.percentile(d_to_truth, 95):.1f} m "
          "(high values here are shore the picture invented)")
    for name in ("building", "road", "paving"):
        k = meta["classes"].index(name)
        m = cls == k
        if m.sum() < 50:
            continue
        ring = (distance_transform_edt(~m) < 6 * per_m) & ~m & ~truth_water      # the ground just around it
        contrast = float(np.linalg.norm(rgb[m].mean(axis=0) - rgb[ring].mean(axis=0)))
        print(f"{name}: colour contrast with its surroundings {contrast:.0f} of 441 (low means the picture lost it)")


def compose(mapdir, styled, out):
    meta, _ = load(mapdir)
    t = meta["transform"]
    rgb, _ = styled_to_grid(styled, meta)
    img = Image.fromarray(rgb.astype(np.uint8))
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = t["extent"]

    def px(x, y):
        return ((x - x0) * t["px_per_m"], t["height_px"] - (y - y0) * t["px_per_m"])

    for lab in meta["labels"]:
        x, y = px(*lab["pos"])
        d.text((x, y), lab["text"].upper(), fill=(60, 40, 30), stroke_width=3, stroke_fill=(245, 235, 210), anchor="mm", font_size=30)
    for icon in meta["icons"]:
        x, y = px(*icon["pos"])
        r = 9 if icon["type"] == "landmark" else 7
        d.ellipse([x - r, y - r, x + r, y + r], fill=(190, 50, 40) if icon["type"] == "landmark" else (250, 240, 215), outline=(40, 25, 20), width=3)
        if icon["label"]:
            d.text((x + r + 5, y), icon["label"], fill=(40, 25, 20), stroke_width=3, stroke_fill=(245, 235, 210), anchor="lm", font_size=20)
    img.save(out)
    print(f"{out}: {len(meta['icons'])} icons and {len(meta['labels'])} labels drawn from data")


def changed(old, new):
    a = json.loads((Path(old) / "map.json").read_text())["tiles"]["hash"]
    b = json.loads((Path(new) / "map.json").read_text())["tiles"]["hash"]
    diff = sorted(k for k in b if a.get(k) != b[k])
    print(f"{len(diff)} of {len(b)} tiles changed and need repainting: {', '.join(diff) if diff else 'none'}")
    ia = {i["id"]: i for i in json.loads((Path(old) / "map.json").read_text())["icons"]}
    ib = {i["id"]: i for i in json.loads((Path(new) / "map.json").read_text())["icons"]}
    moved = [k for k in ib if k in ia and ia[k]["pos"] != ib[k]["pos"]] + [k for k in ib if k not in ia]
    print(f"{len(moved)} icons added or moved, redrawn from data with no repainting: {', '.join(moved) if moved else 'none'}")


if __name__ == "__main__":
    {"measure": measure, "compose": compose, "changed": changed}[sys.argv[1]](*sys.argv[2:])
