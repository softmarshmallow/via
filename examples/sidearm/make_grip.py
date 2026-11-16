"""Writes the sidearm's leaning grip as a model: models/grip.glb (challenge 46; spec/assets.md 12.5).

    python3.12 examples/sidearm/make_grip.py            # writes models/grip.glb
    python3.12 examples/sidearm/make_grip.py --check    # exits 1 when the file is not what it writes
    via pack accept examples/sidearm/pack.toml          # after writing: pack.facts.toml

A part stands upright (rules.md 6), so a leaning part is a model (spec/assets.md 12.5): here a box
the size of the grip whose top stands `LEAN * height` ahead of its foot, as the spike's `add_block`
leaned it (`lean = 0.28`: x moves 0.28 per unit of height). Written by tools/packgen's `glb`, so it
is the starter pack's kind of model: glTF's convention, +Y up, front +Z, so the asset's x (toward
the muzzle) is the model's -x and the asset's y the model's z. Centred in plan on its origin, its
foot at 0, so the pack's `at` for the grip is the leaning grip's middle, where the upright box
stood. Standard library and packgen only.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "tools" / "packgen"))

import packgen  # noqa: E402

LENGTH, WIDTH, HEIGHT = 46, 25, 88  # mm: the upright grip's box (pack.toml before M5's close)
LEAN = 0.28                         # the spike's: x per unit of height


def grip():
    shear = round(LEAN * HEIGHT)
    total = LENGTH + shear
    a = -(total // 2)                      # the foot's back, from the origin
    b = a + LENGTH                         # the foot's front
    w0, w1 = -(WIDTH // 2), WIDTH - WIDTH // 2
    m = 1000.0
    box = packgen.box(-b / m, 0.0, w0 / m, -a / m, HEIGHT / m, w1 / m)
    s, h = shear / m, HEIGHT / m
    # the top moves toward the asset's +x, the model's -x
    tris = [tuple((v[0] - s * v[1] / h, v[1], v[2]) for v in tri) for tri in box]
    return packgen.glb([("bark", tris)])


def main():
    want = {"models/grip.glb": grip()}
    if "--check" in sys.argv[1:]:
        stale = [k for k, t in want.items() if not (HERE / k).exists() or (HERE / k).read_bytes() != t]
        if stale:
            print("stale, run make_grip.py: " + ", ".join(stale), file=sys.stderr)
            sys.exit(1)
        print("models/ is what make_grip.py writes")
        return
    (HERE / "models").mkdir(exist_ok=True)
    for k, t in want.items():
        (HERE / k).write_bytes(t)
        print(f"wrote {k}")


if __name__ == "__main__":
    main()
