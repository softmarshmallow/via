import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write('[{"code": "flat", "hint": "none", "message": "flat ground", "severity": "info", "subject": "field.height"}]')
