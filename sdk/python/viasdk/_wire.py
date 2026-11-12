"""Framing and the connection (protocol.md 3.1).

One frame: ``Content-Length: <n>\\r\\n\\r\\n`` then n bytes of UTF-8 JSON, no other header. The
protocol is read on a thread of its own, so a step blocked in an engine call still sees
``$/cancelRequest``, and the process ends when its input ends, which is when the engine has died.
"""

import json
import os
import queue
import sys
import threading

from ._errors import RpcError

MAX_FRAME = 16 * 1024 * 1024
_PREFIX = b"Content-Length: "


class FramingError(Exception):
    pass


def read_frame(f):
    """Reads one frame body from a binary reader; None at a clean end of input."""
    line = f.readline(64)
    if not line:
        return None
    if not (line.startswith(_PREFIX) and line.endswith(b"\r\n")):
        raise FramingError(f"not a frame header: {line!r}")
    digits = line[len(_PREFIX) : -2]
    if not digits or not digits.isdigit():
        raise FramingError(f"bad Content-Length: {line!r}")
    n = int(digits)
    if f.readline(3) != b"\r\n":
        raise FramingError("no blank line after the header")
    if n > MAX_FRAME:
        raise FramingError(f"frame of {n} bytes exceeds 16 MiB")
    body = f.read(n)
    if len(body) != n:
        raise EOFError("input ended inside a frame")
    return body


def encode(msg):
    """One message as a frame. ``allow_nan=False``: a non-finite number never reaches the wire."""
    body = json.dumps(msg, allow_nan=False, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    if len(body) > MAX_FRAME:
        raise ValueError("message over 16 MiB; send the data as a blob")
    return _PREFIX + str(len(body)).encode("ascii") + b"\r\n\r\n" + body


def _exit():
    """Ends the process at once: nothing can be answered any more (protocol.md 3.1)."""
    try:
        sys.stderr.flush()
    finally:
        os._exit(0)


class Connection:
    """Both directions of the protocol over two binary file objects.

    The reader thread routes responses to the request waiting for them and queues the engine's
    requests and notifications in ``inbox`` for the main loop; ``$/cancelRequest`` is recorded at
    once, since the main thread may be busy in the step.
    """

    def __init__(self, rfile, wfile):
        self._r = rfile
        self._w = wfile
        self._wlock = threading.Lock()
        self._plock = threading.Lock()
        self._next = 0
        self._pending = {}
        self.inbox = queue.Queue()
        self.cancelled = set()
        self.closed = False
        self.busy = False

    def start(self):
        t = threading.Thread(target=self._read_loop, name="viasdk-protocol", daemon=True)
        t.start()

    def _read_loop(self):
        try:
            while True:
                body = read_frame(self._r)
                if body is None:
                    break
                msg = json.loads(body)
                if "method" in msg:
                    if msg["method"] == "$/cancelRequest":
                        self.cancelled.add((msg.get("params") or {}).get("id"))
                        continue
                    self.inbox.put(msg)
                else:
                    with self._plock:
                        slot = self._pending.get(msg.get("id"))
                    if slot is not None:
                        slot[1] = msg
                        slot[0].set()
        except Exception as e:  # noqa: BLE001 - any fault on the wire ends the worker
            print(f"viasdk: protocol input failed: {e!r}", file=sys.stderr)
        # The input has ended: the engine is gone, or it sent its last message (quit). Messages
        # already queued are still served, then the main loop ends at the None. A step that is
        # running, or waiting on an answer that can no longer come, ends the process now.
        self.closed = True
        self.inbox.put(None)
        if self.busy:
            _exit()

    def busy_with(self, flag):
        self.busy = flag
        if flag and self.closed:
            _exit()

    def send(self, msg):
        data = encode(msg)
        with self._wlock:
            self._w.write(data)
            self._w.flush()

    def respond(self, id_, result=None, error=None):
        msg = {"jsonrpc": "2.0", "id": id_}
        if error is not None:
            msg["error"] = error
        else:
            msg["result"] = result
        self.send(msg)

    def request(self, method, params):
        """Sends a request and blocks until its response; raises RpcError on an error answer."""
        ev = threading.Event()
        slot = [ev, None]
        with self._plock:
            self._next += 1
            id_ = self._next
            self._pending[id_] = slot
        try:
            self.send({"jsonrpc": "2.0", "id": id_, "method": method, "params": params})
            while not ev.wait(0.25):
                if self.closed:
                    _exit()
        finally:
            with self._plock:
                self._pending.pop(id_, None)
        msg = slot[1]
        if "error" in msg:
            e = msg["error"]
            raise RpcError(e.get("code"), e.get("message", ""), e.get("data"))
        return msg.get("result")
