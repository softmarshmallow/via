"""Errors a step sees. An error the engine answers is not by itself a diagnostic (protocol.md 3.4):
it is raised, and the step decides; uncaught, it becomes protocol.raised."""


class RpcError(Exception):
    """The engine refused a call: ``code`` is the wire code of protocol.md 3.4."""

    def __init__(self, code, message, data=None):
        super().__init__(f"{message} (code {code})")
        self.code = code
        self.message = message
        self.data = data


class Cancelled(Exception):
    """The engine cancelled the pending step.call ($/cancelRequest). Raised at the step's next
    engine call; letting it propagate answers the call with -32800."""


class NotAWorker(Exception):
    """A compute (expr.eval) asked of a plain command, which has no engine to ask."""
