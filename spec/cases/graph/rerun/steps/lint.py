import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write("[]")
import time
open(os.path.join(request["out"], "time.txt"), "w").write(str(time.time_ns()))
