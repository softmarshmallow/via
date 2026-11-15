# One plaza per given square inside this call's district (composition.md 12).


def generate(step):
    found = step.features("given:squares", within=step.for_each_item)
    for k, name in enumerate(found.names):
        x, y = (int(v) for v in found.points(k)[0])
        step.out.feature("p_" + name.replace(".", "_"), "plaza", at=(x, y))
