import json


def lint(step):
    # A read the engine refuses while no field `extra` exists; the step catches it and goes on.
    try:
        step.call("build.field", fields=["extra"])
        found = "found"
    except Exception:
        found = "refused"
    step.call(
        "diag.emit",
        severity={"dtype": "str", "values": ["warning"]},
        code={"dtype": "str", "values": [found]},
        subject={"dtype": "str", "values": ["world"]},
        message={"dtype": "str", "values": ["the read of `extra` was " + found]},
        hint={"dtype": "str", "values": ["graph.md conformance"]},
        data={"dtype": "str", "values": [json.dumps({})]},
        explain={"dtype": "str", "values": ["A measure the graph chapter's conformance case reads."]},
    )
