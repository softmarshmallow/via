import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write('[]')
open(os.path.join(request["out"], "document.toml"), "w").write("[feature.x]\nat = [1, 1]\n")
