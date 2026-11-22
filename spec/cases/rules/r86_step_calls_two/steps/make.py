def generate(step):
    # Every call of a `for_each` step is the one step (rules.md 3.4, R64, R69): the instance the
    # call for `a` returns and the one the call for `b` returns are two of one step's document.
    if step.for_each_item == "a":
        step.out.instance("a_post", "box_small", at=(6000, 32000))
    else:
        step.out.instance("b_post", "box_small", at=(6400, 32000))
