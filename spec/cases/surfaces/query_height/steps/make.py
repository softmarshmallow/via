def generate(step):
    # One floor at 13.3 over ground at 10. `via query` runs no step, so this floor is not in its
    # answers, and a height that would choose it stands on the terrain there (surfaces.md 7).
    step.out.feature("loft", "storey", region=[(20000, 20000), (32000, 20000), (32000, 28000), (20000, 28000)],
                     level=13.3)
