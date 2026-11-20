def generate(step):
    # Every call of a `for_each` step is the one step (rules.md 3.4, R64): the run the call for
    # `a` returns does not give way to the instance the call for `b` returns.
    if step.for_each_item == "a":
        step.out.table("run", "a_row", asset="box_small", count=4, on={"ref": "a"})
    else:
        step.out.instance("b_post", "box_small", at=(6400, 32000))
