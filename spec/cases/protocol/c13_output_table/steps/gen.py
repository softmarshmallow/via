# A plain command in another language writes TOML itself (protocol.md 5).
import json
import os

with open(os.environ["VIA_STEP"], encoding="utf-8") as f:
    request = json.load(f)
with open(os.path.join(request["out"], "document.toml"), "w", encoding="utf-8") as f:
    f.write('[pin.x]\nasset = "a"\nat = [1, 1]\n')
