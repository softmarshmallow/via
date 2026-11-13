import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
tags = request["data"]["fields"]["height"]["tags"]
diags = [{"code": "tag_" + t, "severity": "warning", "subject": "field.height",
          "message": "height is tagged " + t, "hint": "graph.md 5.1",
          "explain": "The lint names each tag of the field height."} for t in tags]
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write(json.dumps(diags))
