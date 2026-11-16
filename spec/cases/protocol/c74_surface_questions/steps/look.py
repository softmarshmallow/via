def lint(step):
    at = step.surfaces_at([1000, 4000, 9000], [1000, 4000, 1000])
    k = step.sample("deck.top", [4000, 6000, 7000], [4000, 4000, 4000])
    t = step.sample("height", [4500], [1000])
    step.info("asked", "step.look", "asked surfaces_at and sample", "nothing to do",
              data={"point": [int(v) for v in at["point"]], "surface": list(at["surface"]),
                    "z": [float(v) for v in at["z"]],
                    "inside": [bool(v) for v in k["inside"]], "kz": [float(v) for v in k["z"]],
                    "kgx": [float(v) for v in k["gx"]], "kgy": [float(v) for v in k["gy"]],
                    "tz": [float(v) for v in t["z"]], "tgx": [float(v) for v in t["gx"]]},
              explain="The surface questions' answers, as the engine gave them.")
