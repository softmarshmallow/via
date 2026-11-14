def generate(step):
    # Reads the ground first, then the point `p`, and returns a feature 1 m east of the point.
    step.fields(["height"])
    got = step.call("build.features", select=["p"], columns=[])
    flat = got["coords"]["values"]
    x, y = (flat[0] if flat and isinstance(flat[0], list) else flat[:2])[:2]
    step.out._tables.append("[feature.found]\nat = [%s, %s]\n" % ((int(x) + 1000) / 1000, int(y) / 1000))
