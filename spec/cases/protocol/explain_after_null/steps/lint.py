import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write('[{"code": "steep", "data": {"slope": 0.62}, "explain": "The bank is steeper than a ford allows.", "hint": "Lower the bank or move the ford.", "message": "Slope 0.62 exceeds 0.5.", "position": [8, 8], "severity": "warning", "subject": "field.height"}]')
