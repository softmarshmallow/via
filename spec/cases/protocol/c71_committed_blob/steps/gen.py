def generate(step):
    # One derive.spill batch of 5000 points, more than a column may carry inline (protocol.md 4.2:
    # 4096), so the request names its columns by blob.
    n = 5000
    xs = [(k % 8) * 1000 for k in range(n)]
    ys = [((k // 8) % 8) * 1000 for k in range(n)]
    step.spill("height", xs, ys)
    step.out.feature("spot", "mark", at=[4000, 4000])
