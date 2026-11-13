import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import helper
import json, os
request = json.load(open(os.environ["VIA_STEP"], encoding="utf-8"))
open(os.path.join(request["out"], "diagnostics.json"), "w", encoding="utf-8").write('[]')
