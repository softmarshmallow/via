import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
name = request["data"]["instances"]["name"]["values"][0]
out = [{"code": "near", "severity": "info", "subject": name, "message": "a tree", "hint": "none", "explain": "A tree."}]
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write(json.dumps(out))
