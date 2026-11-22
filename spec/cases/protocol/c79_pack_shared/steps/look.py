import json


def lint(step):
    got = step.pack("*", columns=["params", "shared"])
    shared = {i: (json.loads(s) if s else None) for i, s in zip(got["id"], got["shared"])}
    step.info("read", "step.look", "read what each asset's kit states", "nothing to do",
              data={"ids": list(got["id"]),
                    "module": [(shared[i] or {}).get("module", 0) for i in got["id"]],
                    "wall": [(shared[i] or {}).get("wall", 0) for i in got["id"]],
                    "none": [i for i in got["id"] if shared[i] is None]},
              explain="The `shared` column of pack.query, read for every asset.")
