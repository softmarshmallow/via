"""The worker loop (protocol.md 3.2): initialize, step.call, shutdown, quit.

Runs after the launcher has taken the descriptors. One step.call at a time; the step function runs
on the main thread while the protocol is read on its own thread (see _wire).

The order is protocol.md 3.1 and 8's: take the descriptors, answer initialize, then import the
script, at the first step.call. So the script's import time (NumPy or heavier) never counts against
initialize's 10 s deadline (6.2), and an import error is answered where 3.4 defines -32010: to
step.call, as protocol.raised. initialize needs nothing from the script, since capabilities.when
lists every point this SDK dispatches.
"""

import importlib.util
import os
import sys
import traceback

from . import PROTOCOL, __version__
from ._errors import Cancelled
from ._step import Step
from ._wire import Connection

_POINTS = ("generate", "lint", "export")


def _load(script):
    """Imports the script as a module, its own folder first on the path (protocol.md 6.1:
    PYTHONSAFEPATH keeps the working folder off it)."""
    path = os.path.abspath(script)
    sys.path.insert(0, os.path.dirname(path))
    spec = importlib.util.spec_from_file_location("__via_step__", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules["__via_step__"] = module
    spec.loader.exec_module(module)
    return module


def _raised(e):
    tb = "".join(traceback.format_exception(e))
    print(tb, file=sys.stderr, end="")
    return {
        "code": -32010,
        "message": f"the step raised {type(e).__name__}",
        "data": {"exception": type(e).__name__, "traceback": tb},
    }


class _Script:
    """The step's script, imported once, at the first step.call. An import that failed fails every
    call the same way; it is not retried, so every call of the build sees one script."""

    def __init__(self, path):
        self._path = path
        self._done = False
        self.module = None
        self.error = None

    def load(self):
        if not self._done:
            self._done = True
            try:
                self.module = _load(self._path)
            except BaseException as e:  # noqa: BLE001 - reported to the engine, not swallowed
                self.error = e
        return self.module, self.error


def serve(script, rfile, wfile):
    conn = Connection(rfile, wfile)
    conn.start()
    script = _Script(script)

    while True:
        msg = conn.inbox.get()
        if msg is None:
            return 0
        method, id_ = msg.get("method"), msg.get("id")
        params = msg.get("params") or {}
        if method == "initialize":
            if params.get("protocol") != PROTOCOL:
                conn.respond(id_, error={
                    "code": -32001,
                    "message": f"this SDK speaks {PROTOCOL}",
                    "data": {"protocol": PROTOCOL},
                })
                sys.stderr.flush()
                os._exit(1)
            # capabilities.when is what this SDK can dispatch, all three points, not what the
            # script defines: a missing function is protocol.no_entry at step.call (protocol.md
            # 3.2), so one condition has one code.
            conn.respond(id_, {
                "protocol": PROTOCOL,
                "sdk": {"name": "viasdk", "version": __version__},
                "capabilities": {"when": list(_POINTS)},
            })
        elif method == "step.call":
            # Busy from before the import: an engine that dies during a slow import ends the
            # process too (_wire).
            conn.busy_with(True)
            module, error = script.load()
            if error is not None:
                reply = {"error": _raised(error)}
            else:
                reply = _call(module, conn, id_, params)
            conn.busy_with(False)
            conn.respond(id_, **reply)
        elif method == "shutdown":
            conn.respond(id_, None)
        elif method == "quit":
            sys.stderr.flush()
            os._exit(0)
        elif id_ is not None:
            conn.respond(id_, error={"code": -32601, "message": f"unknown method {method}",
                                     "data": {"method": method}})


def _call(module, conn, id_, request):
    """One step.call: the step function's result, or the wire error it ends in."""
    when = request.get("when")
    fn = getattr(module, when, None) if when in _POINTS else None
    if not callable(fn):
        return {"error": {"code": -32601, "message": f"the script has no function {when}",
                          "data": {"code": "protocol.no_entry", "when": when}}}
    step = Step(request, conn, id_)
    try:
        try:
            fn(step)
        finally:
            # What the step said before it returned or raised is sent in one batch (R23).
            if id_ not in conn.cancelled:
                step.flush()
    except Cancelled:
        return {"error": {"code": -32800, "message": "cancelled"}}
    except Exception as e:  # noqa: BLE001 - the worker stays up for the next call (3.4)
        if id_ in conn.cancelled:
            return {"error": {"code": -32800, "message": "cancelled"}}
        return {"error": _raised(e)}
    if id_ in conn.cancelled:
        return {"error": {"code": -32800, "message": "cancelled"}}
    if when == "generate":
        text = step.out.text()
        if not text and os.path.exists(os.path.join(step.out.folder, "document.toml")):
            # The script wrote document.toml itself (protocol.md 5): the engine reads it there.
            return {"result": {"document": None}}
        return {"result": {"document": text}}
    return {"result": None}
