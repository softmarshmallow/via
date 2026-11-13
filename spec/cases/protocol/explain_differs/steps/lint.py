import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write('[{"code": "flat", "explain": "one text", "hint": "none", "message": "flat ground", "position": [2, 2], "severity": "info", "subject": "field.height"}, {"code": "flat", "explain": "another text", "hint": "none", "message": "flat ground", "position": [4, 4], "severity": "info", "subject": "field.height"}]')
