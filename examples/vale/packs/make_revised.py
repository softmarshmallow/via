"""Writes packs/revised/house_low.glb: the starter pack's `house_low` re-exported half a metre
wider each way (9 m by 6 m walls in place of 8 m by 6 m), for the change `revised_house`.

    python3 examples/vale/packs/make_revised.py

The model is made by tools/packgen's own `house`, so it differs from the shipped one only in width.
`packs/revised.facts.toml` is the starter pack's facts file as accepted before the re-export, so
the build compares the new model with the old facts and names every instance of the asset
(assets.md 4.2, 11).
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

(HERE / "revised").mkdir(exist_ok=True)
(HERE / "revised" / "house_low.glb").write_bytes(packgen.glb(packgen.house(9.0, 6.0, 3.0, 4.6)))
