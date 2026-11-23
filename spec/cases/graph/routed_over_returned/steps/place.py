"""Returns a mark on the straight line from p_start to p_end, and reads nothing."""


def generate(step):
    step.out.feature("block", "mark", at=(8000, 8000))
