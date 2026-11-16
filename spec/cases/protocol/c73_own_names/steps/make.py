def generate(step):
    # Names inside the returned document resolve to its own keys first (protocol.md 5, Names):
    # `q.start` is `make#q.start` and `q` is `make#q`.
    step.out.feature("q", "way", path=[(3000, 1000), (6000, 1000)])
    step.out.feature("p", "way", path=[(1000, 1000), (3000, 1000)], ends_at="q.start")
    step.out.feature("t", "tag", at=(5000, 5000), host="q")
