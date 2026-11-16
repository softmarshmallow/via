"""The atlas's roads and trails in earth on the plain's `ground` (examples/atlas/layers/ways.toml).

A row on `ground` must be of the ways layer's rank to see its roads (documents.md 5.1: a formula
refers to its layer and below), and a layer may not write a row on a field another document
declares (`identity.duplicate_id`). A step of the ways layer may (fields.md 7: a step returns rows
on any field in its stratum, at its layer's rank), so this one returns the row the layer gives it
in `params.mask`, unread. A worker that reads nothing: it reads no stated fact (derived.md 5,
so no lint need check it), and only an edit of its params runs it again.
"""


def generate(step):
    step.out.row("ground", "track", paint="earth", mask=step.params["mask"])
