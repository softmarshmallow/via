def lint(step):
    # A warning whose subject is the feature the generate step returned.
    step.warning("returned", "feature.gen#found", "the returned feature is here",
                 "nothing to do", explain="The lint names a feature a step returned.")
