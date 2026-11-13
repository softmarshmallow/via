import json, os
raw = open(os.environ["VIA_STEP"], encoding="utf-8").read()
request = json.loads(raw, parse_float=str, parse_int=str)
want = {'a': '1000.5', 'b': '1.0', 'c': '255', 'd': '15', 'e': '5', 'f': '9007199254740993', 'g': '6.0', 'h': '6'}
bad = [k for k, v in want.items() if request["params"].get(k) != v]
diags = []
if bad:
    diags.append({"code": "params", "severity": "error", "subject": "field.height", "message": "params differ: " + ", ".join(bad),
                  "hint": "protocol.md 2.1", "explain": "The request's params text differs from protocol.md 2.1's table."})
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write(json.dumps(diags))
