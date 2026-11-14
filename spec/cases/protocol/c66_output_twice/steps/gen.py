# Returns its document in the call's result (the SDK's step.out) and writes out/document.toml too.
import os


def generate(step):
    step.out.feature("k", "mark", at=(1000, 1000))
    with open(os.path.join(step.request["out"], "document.toml"), "w", encoding="utf-8") as f:
        f.write("")
