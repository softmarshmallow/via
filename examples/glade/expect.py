"""Glade's measures, read from its build directories and held to expect.toml (Python 3.11+, NumPy).

    python expect.py BUILD_ROOT     # BUILD_ROOT holds glade/, glade+mouth/, ... as expect.toml names

Prints one line per measure, `ok` or `OUT`, with its range and the spike's value where there is one,
and exits 1 when any measure is out. A build directory that is missing is reported and skipped.
"""

import json
import sys
import tomllib
from collections import Counter
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
out = []


def say(ok, what, value, want, oracle=None):
    o = "" if oracle is None else f"  (spike {oracle})"
    out.append(ok)
    print(f"{'ok ' if ok else 'OUT'}  {what}: {value}  want {want}{o}")


def within(v, r):
    return r[0] <= v <= r[1]


def load(d):
    m = json.loads((d / "manifest.json").read_text())
    names = (d / "instances/names.txt").read_text().split("\n")[:-1]
    diags = json.loads((d / "diagnostics.json").read_text())
    changes = json.loads((d / "changes.json").read_text())
    run = d.with_name(d.name + ".run.json")
    run = json.loads(run.read_text()) if run.exists() else None
    return m, names, diags, changes, run


def producer(name):
    return name.split("#")[0] if "#" in name else None


def check_counts(e, names, diags):
    c = diags["counts"]
    say(within(c["error"], e["errors"]), "errors", c["error"], e["errors"])
    if "warnings" in e:
        say(within(c["warning"], e["warnings"]), "warnings", c["warning"], e["warnings"])
    by = Counter(producer(n) for n in names)
    if "instances" in e:
        i = e["instances"]
        say(within(len(names), i["range"]), "instances", len(names), i["range"], i.get("oracle"))
    for p in ("trees", "bushes", "stones"):
        if p in e:
            say(within(by[p], e[p]["range"]), p, by[p], e[p]["range"])


def scattered_share(changes):
    kept = total = 0
    for s in changes["summary"]:
        if s.get("maker") is not None or s["table"] not in ("feature", "field", "footprint", "instance"):
            kept += s["untouched"]
            total += s["untouched"] + s["removed"] + s["moved"] + s["changed"]
    return kept / total if total else 1.0


def main(root):
    exp = tomllib.loads((HERE / "expect.toml").read_text())
    builds = {}
    for name, e in exp.items():
        d = root / e["build"]
        if not (d / "manifest.json").exists():
            print(f"--   {name}: no build at {d}, skipped")
            continue
        print(f"== {name} ({d})")
        m, names, diags, changes, run = builds[name] = load(d)
        check_counts(e, names, diags)
        if "ground" in e:
            h = np.load(d / "fields/height.npy")
            r = m["frame"]["resolution"]
            for g in e["ground"]:
                x, y = g["at"]
                v = float(h[int(round(y / r)), int(round(x / r))])
                say(abs(v - g["oracle"]) <= g["within"], f"ground at {g['at']}", round(v, 2),
                    f"within {g['within']} m", g["oracle"])
        if "scattered_untouched_share" in e:
            s = e["scattered_untouched_share"]
            v = round(scattered_share(changes), 3)
            say(within(v, s["range"]), "scattered untouched share", v, s["range"])
        entries = changes["entries"]
        if "field_box_min_x" in e:
            xs = [en["region"]["box"][0] for en in entries if en["table"] == "field" and en["region"]]
            v = min(xs) if xs else None
            say(v is not None and v >= e["field_box_min_x"], "west edge of changed fields", v,
                f">= {e['field_box_min_x']}")
        if "moved_features" in e:
            got = sorted(en["name"] for en in entries if en["table"] == "feature" and "moved" in en["what"])
            say(got == sorted(e["moved_features"]), "moved features", got, sorted(e["moved_features"]))
        if "fields_changed" in e:
            got = sorted(en["name"] for en in entries if en["table"] == "field")
            say(got == sorted(e["fields_changed"]), "fields changed", got, sorted(e["fields_changed"]))
        if "entries" in e:
            got = sorted(f"{en['table']}:{en['name']}" for en in entries)
            say(got == sorted(e["entries"]), "change report entries", got, sorted(e["entries"]))
        if "ran_producers" in e and run is not None:
            lints = {f"step:{x['step']}" for x in diags["lints"]}
            got = sorted(n["id"] for n in run["nodes"] if n["status"] == "ran"
                         and n["id"].split(":")[0] in ("rule", "step") and n["id"] not in lints)
            say(got == e["ran_producers"], "producers that ran", got, e["ran_producers"])
        if "revised" in e:
            rv = e["revised"]
            ds = [x for x in diags["diagnostics"] if x["code"] == "asset.revised"
                  and x["subject"] == f"asset.{rv['asset']}"]
            n = len(ds[0]["data"]["instances"]) if ds else 0
            base = builds.get("base")
            want = sum(c["instances"].get(rv["asset"], [0, 0])[1] for c in base[0]["chunks"]) if base else None
            say(ds != [] and n == want, f"asset.revised {rv['asset']} instances", n, want)
    return 0 if all(out) else 1


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]) if len(sys.argv) > 1 else HERE / "build"))
