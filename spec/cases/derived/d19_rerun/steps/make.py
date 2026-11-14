def generate(step):
    step.call("build.features", select=["haven"], columns=[])
    # The SDK's feature writer is M2 work; the document is TOML text either way (protocol.md 5).
    step.out._tables.append("[feature.mark]\nat = [1, 1]\n")
