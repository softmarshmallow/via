"""Lints: checks that run only when asked for (`--lint NAME`), because they cost time or need the
author to have stated expectations first. Each lint is one module here; `reach` is the only one.

`LINTS` is the one list of them. The command line and the pipeline both read it, so a name that is
not here is refused instead of being quietly skipped."""
from .reach import lint_reach

# name -> (the lint, the picture it leaves in the output folder)
LINTS = {"reach": (lint_reach, "reach.png")}
