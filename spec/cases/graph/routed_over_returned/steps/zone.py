"""Returns a mark at (8, 8), and one at the first point of `zone`, or at (2, 2) where it has none:
what the step read of the region is in what it returns."""


def generate(step):
    step.out.feature("block", "mark", at=(8000, 8000))
    z = step.features(select="zone")
    try:
        x, y = z.points(0)[0]
    except Exception:
        x, y = 2000, 2000
    step.out.feature("far", "mark", at=(int(x), int(y)))
