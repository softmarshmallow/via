# 0006 — How does this differ from Houdini-style terrain tools?

**Status: doctrine.**

**Q:** In Houdini or similar, must the designer intentionally place an
oxbow to ever get that geomorphology?

**A:** Yes. Houdini's HeightField Erode (and Gaea/World Machine peers)
run droplet/thermal erosion — a *texture-scale* operator over effective
seconds-to-centuries. The river **course is an input**: draw a spline,
carve, erode around it. No lateral migration exists in the node graph,
so meanders never amplify and necks never cut off; an oxbow is drawn and
dressed by hand. Meander-shaped spline generators exist, but that is
authored shape wearing a procedural costume.

The division of labor is inverted relative to via: DCC tools **author the
macroform and simulate the microtexture**; via simulates the macroform
from declared forcing and refuses to author anything derivable
(ADR 0003). Their designers must know geomorphology well enough to fake
it correctly (oxbows only in low-gradient floodplain reaches, scroll bars
wrapping the right way); a process model cannot make that class of error.

The graphics research frontier is converging on our side: recent
SIGGRAPH-line work simulates meandering with the same kinematic
centerline models we plan to use, with authoring handles on top —
repeating what happened when graphics adopted stream-power erosion over
pure noise a decade ago.

The two worlds compose: via's artifacts import cleanly into Houdini
heightfields, so DCC tools are excellent *consumers* (meshing,
scattering, look-dev) downstream of a causally sound substrate.
