# Returns a key that is not an id (identity.md 2.2).
import json
import os

with open(os.environ["VIA_STEP"], encoding="utf-8") as f:
    request = json.load(f)
with open(os.path.join(request["out"], "document.toml"), "w", encoding="utf-8") as f:
    f.write('[feature.Post_1]\nkind = "mark"\nat = [1, 1]\n')
