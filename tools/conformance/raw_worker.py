"""The raw worker (conformance.md 3.7): a scripted worker that speaks via-step-1 itself, on
descriptors 0 and 1, without the SDK, and breaks one rule of `protocol.md` on request.

A case copies it in as a step program and names the fault as the step's first argument:

    command = ["python", "steps/raw.py", "blob_digest"]
    worker  = true

| Fault | What the worker does |
|---|---|
| `none` | Nothing wrong: lists every point, answers each `step.call` with an empty document, and exits on `shutdown` and `quit`. |
| `when_unserved` | Lists only `export` in `capabilities.when` (3.2). |
| `message_too_large` | Answers `step.call` with one frame of 16 MiB and one byte (3.1). |
| `blob_digest` | Returns its document as a blob named by another digest: the bytes of `# raw\n` at the name of `# other\n` (4.1). |
| `blob_header` | Calls `expr.eval` with an `i32` column `x` of shape `[2]` whose blob, named by its own BLAKE3 digest, holds 4 bytes, not 8 (4.2); then fails the call as the SDK would, with -32010. |
| `slow_exit` | Serves as `none`, then answers neither `shutdown` nor `quit` and stays alive 6 s, past the 2 s deadline (6.2). |

It writes nothing to descriptor 1 but frames. A fault it does not know ends it with status 2 before
`initialize` is answered.
"""

import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

PROTOCOL = "via-step-1"
FAULTS = ("none", "when_unserved", "message_too_large", "blob_digest", "blob_header", "slow_exit")
LIMIT = 16 * 1024 * 1024

fault = sys.argv[1] if len(sys.argv) > 1 else "none"
if fault not in FAULTS:
    sys.stderr.write(f"raw worker: unknown fault {fault!r}; one of {', '.join(FAULTS)}\n")
    sys.exit(2)

wire_in = sys.stdin.buffer
wire_out = sys.stdout.buffer


def read():
    """One frame, or None when the engine closed the input."""
    length = None
    while True:
        line = wire_in.readline()
        if not line:
            return None
        if line in (b"\r\n", b"\n"):
            break
        name, _, value = line.decode("ascii").partition(":")
        if name.strip().lower() == "content-length":
            length = int(value.strip())
    return json.loads(wire_in.read(length))


def send(msg, pad_to=None):
    body = json.dumps(msg, separators=(",", ":")).encode("utf-8")
    if pad_to is not None:
        body = body[:-1] + b" " * (pad_to - len(body)) + b"}"
    wire_out.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    wire_out.flush()


def put_blob(objects, data, name):
    """Writes `data` at `<objects>/ab/<name>` by the no-replace move of protocol.md 4.1."""
    tmp = os.path.join(objects, "tmp", f"raw-{os.getpid()}-{name[:16]}")
    os.makedirs(os.path.dirname(tmp), exist_ok=True)
    with open(tmp, "wb") as fh:
        fh.write(data)
    dest = os.path.join(objects, name[:2], name)
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    try:
        os.link(tmp, dest)
    except FileExistsError:
        pass
    os.unlink(tmp)
    return dest


objects = None
own_id = 0


def call_engine(method, params):
    """A worker-to-engine request; returns the engine's response (other messages are dropped)."""
    global own_id
    own_id += 1
    send({"jsonrpc": "2.0", "id": own_id, "method": method, "params": params})
    while True:
        msg = read()
        if msg is None:
            sys.exit(0)
        if msg.get("id") == own_id and "method" not in msg:
            return msg


def step_call(msg):
    cid = msg["id"]
    if fault == "message_too_large":
        send({"jsonrpc": "2.0", "id": cid, "result": {"document": ""}}, pad_to=LIMIT + 1)
        return
    if fault == "blob_digest":
        import blake3
        other = blake3.digest(b"# other\n")
        put_blob(objects, b"# raw\n", other)
        send({"jsonrpc": "2.0", "id": cid, "result": {"document": {"blob": other}}})
        return
    if fault == "blob_header":
        import blake3
        data = (0).to_bytes(4, "little")
        name = blake3.digest(data)
        path = put_blob(objects, data, name)
        answer = call_engine("expr.eval", {
            "call": cid, "formula": "= x",
            "x": {"dtype": "i32", "shape": [2], "blob": name, "path": path.replace(os.sep, "/")},
            "y": {"dtype": "i32", "values": [0, 0]},
        })
        err = answer.get("error") or {"code": 0, "message": "the engine accepted the column"}
        sys.stderr.write(f"raw worker: expr.eval answered {json.dumps(err)}\n")
        send({"jsonrpc": "2.0", "id": cid, "error": {
            "code": -32010, "message": f"engine error {err['code']}: {err['message']}",
            "data": {"exception": "ViaError", "traceback": ""}}})
        return
    send({"jsonrpc": "2.0", "id": cid, "result": {"document": ""}})


while True:
    msg = read()
    if msg is None:
        if fault == "slow_exit":
            time.sleep(6)
        sys.exit(0)
    method = msg.get("method")
    if method == "initialize":
        objects = msg["params"]["objects"]
        when = ["export"] if fault == "when_unserved" else ["export", "generate", "lint"]
        send({"jsonrpc": "2.0", "id": msg["id"], "result": {
            "protocol": PROTOCOL, "sdk": {"name": "raw", "version": "0"},
            "capabilities": {"when": when}}})
    elif method == "step.call":
        step_call(msg)
    elif method == "shutdown":
        if fault == "slow_exit":
            time.sleep(6)
            sys.exit(0)
        send({"jsonrpc": "2.0", "id": msg["id"], "result": None})
    elif method == "quit":
        sys.exit(0)
    elif "id" in msg and method is not None:
        send({"jsonrpc": "2.0", "id": msg["id"], "error": {"code": -32601, "message": f"no method {method}"}})
