import numpy as np

def lint(step):
    (f,) = step.fields(["mix"])
    v = step.eval("= 0.5 * x + y * y / 3 + slope('height')", window=[0, 0, 9, 9])
    got = np.asarray(v, dtype=np.float64).astype(np.float32).reshape(9, 9)
    step.info("compared", "step.look", "compared expr.eval with the built field", "nothing to do",
              data={"same": bool(np.array_equal(got, np.asarray(f))), "n": int(v.size)},
              explain="The worker compared expr.eval over the whole grid with the field the same formula builds.")
