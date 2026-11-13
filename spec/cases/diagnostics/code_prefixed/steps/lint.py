import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write(json.dumps([{'code': 'unknown_name', 'severity': 'warning', 'subject': 'field.height', 'message': 'Too steep.', 'hint': 'Flatten it.', 'explain': 'A steep place.'}]))
