"""A worker that does not use the launcher, for faults the launcher exists to prevent
(protocol.md cases 25 and 27). Speaks the wire by hand on descriptors 0 and 1."""

import json
import os
import sys
import time

from viasdk._wire import encode, read_frame

mode = sys.argv[1]
rin = os.fdopen(0, "rb")


def send(msg):
    os.write(1, encode(msg))


msg = json.loads(read_frame(rin))
if mode == "silent":
    time.sleep(600)
proto = "via-step-2" if mode == "version" else "via-step-1"
when = ["lint"] if mode == "when" else ["generate"]
send({"jsonrpc": "2.0", "id": msg["id"], "result": {
    "protocol": proto, "sdk": {"name": "fake", "version": "0"}, "capabilities": {"when": when}}})
if mode == "hello":
    read_frame(rin)  # the step.call
    os.write(1, b"hello\n")
while read_frame(rin) is not None:
    pass
