# Returns the key `post`, the name of a feature written in the step's stratum (protocol.md 5, Names).
import json
import os

with open(os.environ["VIA_STEP"], encoding="utf-8") as f:
    request = json.load(f)
with open(os.path.join(request["out"], "document.toml"), "w", encoding="utf-8") as f:
    f.write('[feature.post]\nkind = "mark"\nat = [1, 1]\n')
