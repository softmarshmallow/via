"""The launcher: ``python -m viasdk <script>`` (protocol.md 3.1 and 8).

The first thing it does, before any user code is imported, is take the standard descriptors for
the protocol; only then does it import the script. So a ``print`` in user code reaches the log and
never the wire, and a subprocess the user starts reads the null device, never a frame.
"""

import os
import sys


def _take_descriptors():
    """Duplicates descriptors 0 and 1 for the protocol, marks the copies non-inheritable, then
    points 0 at the null device and 1 at standard error, and ``sys.stdin``/``sys.stdout`` with
    them. Returns the protocol's binary reader and writer."""
    proto_in = os.dup(0)
    proto_out = os.dup(1)
    os.set_inheritable(proto_in, False)
    os.set_inheritable(proto_out, False)
    if sys.platform == "win32":
        # Text mode translates line ends on Windows: the failure this rule exists for.
        import msvcrt

        msvcrt.setmode(proto_in, os.O_BINARY)
        msvcrt.setmode(proto_out, os.O_BINARY)
    null = os.open(os.devnull, os.O_RDONLY)
    os.dup2(null, 0)
    os.close(null)
    os.dup2(2, 1)
    sys.stdin = open(os.devnull, encoding="utf-8")
    sys.stdout = sys.stderr
    rfile = os.fdopen(proto_in, "rb", buffering=1 << 16)
    wfile = os.fdopen(proto_out, "wb", buffering=1 << 16)
    return rfile, wfile


def main(argv):
    if len(argv) != 1:
        print("usage: python -m viasdk <script.py>", file=sys.stderr)
        return 2
    if os.environ.get("VIA_STEP"):
        # The plain command (protocol.md 2.2, 8) arrives with M1; the transport is told apart by
        # VIA_STEP alone, so a worker never takes this branch.
        print("viasdk: the plain-command transport arrives with M1", file=sys.stderr)
        return 1
    rfile, wfile = _take_descriptors()
    from ._worker import serve

    return serve(argv[0], rfile, wfile)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
