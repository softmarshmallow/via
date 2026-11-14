# Returns a feature written as anchored: a step finds, the author states (R14).
import json
import os

with open(os.environ["VIA_STEP"], encoding="utf-8") as f:
    request = json.load(f)
with open(os.path.join(request["out"], "document.toml"), "w", encoding="utf-8") as f:
    f.write('[feature.k]\nkind = "mark"\nauthority = "anchored"\nat = [1, 1]\n')
