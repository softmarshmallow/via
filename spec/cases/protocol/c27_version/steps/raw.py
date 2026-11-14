# A worker that does not use the launcher: it speaks the protocol on descriptors 0 and 1 itself.
import json
import sys

PROTOCOL = 'via-step-2'

def read():
    line = sys.stdin.buffer.readline()
    if not line:
        sys.exit(0)
    n = int(line.split(b":")[1])
    sys.stdin.buffer.readline()
    return json.loads(sys.stdin.buffer.read(n))

def write(msg):
    body = json.dumps(msg).encode("utf-8")
    sys.stdout.buffer.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    sys.stdout.buffer.flush()

msg = read()
write({"jsonrpc": "2.0", "id": msg["id"], "result": {
    "protocol": PROTOCOL, "sdk": {"name": "raw", "version": "0"},
    "capabilities": {"when": ["generate", "lint", "export"]}}})
sys.stdout.buffer.write(b"hello\n")
sys.stdout.buffer.flush()
while True:
    read()
