"""A lint step: the pilgrim way is one unbroken line from the map edge to the hall.

The reach lint walks the ground and cannot see a deck, so it cannot say that the bridge joins the two
banks. This rule does: each way in `chain` must begin where the one before it ends, the ground at
the two ends of the deck must be near level, and the last stretch must be lit.
"""
import math

from viasdk import step

s = step()
p = s.params
feats = s.build.features
h = s.build.field("height")
chain = [feats.get(i) for i in p["chain"]]
for name, f in zip(p["chain"], chain):
    if f is None:
        s.error("missing", name, "the way's chain names a feature the build does not have")
chain = [f for f in chain if f is not None]

for a, b in zip(chain, chain[1:]):
    ax, ay = a["points"][-1]
    bx, by = b["points"][0]
    gap = math.hypot(ax - bx, ay - by)
    if gap > p["max_gap_m"]:
        s.error("broken", b["id"], f"it begins {gap:.1f} m from the end of '{a['id']}'; the way allows {p['max_gap_m']} m",
                where=(bx, by), hint="give one of them starts_at / ends_at the other")
    step_m = abs(float(h[s.frame.index(ax, ay)]) - float(h[s.frame.index(bx, by)]))
    if step_m > p["max_step_m"]:
        s.warn("step", b["id"], f"the ground steps {step_m:.1f} m where it meets '{a['id']}'", where=(bx, by))

lamps = [i for i in s.build.explicit if i["id"].startswith(p["lamp_run"] + ".")]
if len(lamps) < p["min_lamps"]:
    s.warn("unlit", p["lamp_run"], f"{len(lamps)} lamps stand on the last stretch; the brief wants at least {p['min_lamps']}")
s.done()
