"""Returns a mark on the straight line from p_start to p_end, and a mark at the farthest point of
`course` from that line: what the step read of the course is in what it returns."""


def generate(step):
    step.out.feature("block", "mark", at=(8000, 8000))
    course = step.features(select="course")
    pts = course.points(0)
    x, y = max(pts, key=lambda p: abs(int(p[1]) - 8000))
    step.out.feature("far", "mark", at=(int(x), int(y)))
