"""viasdk: the Python side of via's step protocol, via-step-1 (spec/protocol.md section 8).

Pure Python. It never links or embeds the engine (R20): a step runs in its own process and speaks
JSON-RPC on stdio, and arrays travel as files named by digest, mapped by both sides.

This module imports nothing heavy and no user code: ``python -m viasdk`` imports it before the
launcher has taken the protocol's descriptors, so it must not print or read standard input.
NumPy is imported only where fields are touched.
"""

# The protocol this SDK speaks, checked exactly by the engine (protocol.md 3.2).
PROTOCOL = "via-step-1"

# Major version = protocol number: viasdk 1.x speaks via-step-1. Recorded in step results, never
# checked (protocol.md 8); the protocol string above is what is checked.
__version__ = "1.0.0a0"

from ._errors import Cancelled, NotAWorker, RpcError  # noqa: E402
from ._step import FeatureList, Step, metres  # noqa: E402
from ._plain import request  # noqa: E402
from ._kit import attrs, ground, kit  # noqa: E402

__all__ = ["PROTOCOL", "Cancelled", "FeatureList", "NotAWorker", "RpcError", "Step", "__version__",
           "attrs", "ground", "kit", "metres", "request"]
