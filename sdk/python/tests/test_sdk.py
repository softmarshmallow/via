"""Unit tests of viasdk's own parts. Run with the SDK's uv environment:

    sdk/python/.venv/bin/python -m unittest discover -s sdk/python/tests

The engine-side probe (crates/via-proto/tests/probe.rs) drives the whole SDK through a real engine.
"""

import io
import json
import os
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest

import numpy as np

SDK = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, SDK)

import viasdk  # noqa: E402
from viasdk import _npy, _wire  # noqa: E402


class Framing(unittest.TestCase):
    def test_round_trip_counts_bytes(self):
        frame = _wire.encode({"é": 1})
        self.assertTrue(frame.startswith(b"Content-Length: 8\r\n\r\n"))
        r = io.BytesIO(frame + _wire.encode({"a": 2}))
        self.assertEqual(json.loads(_wire.read_frame(r)), {"é": 1})
        self.assertEqual(json.loads(_wire.read_frame(r)), {"a": 2})
        self.assertIsNone(_wire.read_frame(r))

    def test_refuses_what_is_not_a_frame(self):
        for bad in [b"hello\n", b"Content-Length: 2\n\n{}", b"Content-Type: x\r\n\r\n{}",
                    b"Content-Length: 99999999\r\n\r\n"]:
            with self.assertRaises(_wire.FramingError, msg=bad):
                _wire.read_frame(io.BytesIO(bad))

    def test_no_nan_on_the_wire(self):
        with self.assertRaises(ValueError):
            _wire.encode({"x": float("nan")})


class FieldFiles(unittest.TestCase):
    def test_header_is_the_engines(self):
        h = _npy.header("<f4", (513, 513))
        self.assertEqual(len(h), 128)
        self.assertIn(b"{'descr': '<f4', 'fortran_order': False, 'shape': (513, 513), }", h)
        self.assertEqual(len(_npy.header("|u1", (2, 3, 4))) % 64, 0)

    def test_written_field_maps_and_has_no_negative_zero(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "f.npy")
            a = np.array([[0.0, -0.0], [1.5, -2.0]], dtype=np.float64)
            _npy.write_field(p, -a)
            m = np.load(p, mmap_mode="r")
            self.assertEqual(m.dtype, np.dtype("<f4"))
            self.assertEqual(m.tolist(), [[0.0, 0.0], [-1.5, 2.0]])
            self.assertFalse(np.signbit(m[0]).any(), "-0.0 is written as +0.0")
            with open(p, "rb") as f:
                self.assertEqual(len(f.read()), 128 + 16)

    def test_bytes_are_little_endian_whatever_the_input(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "f.npy")
            a = np.array([[1.5, -0.0], [2.25, 3.0]], dtype=">f8")
            _npy.write_field(p, a)
            with open(p, "rb") as f:
                body = f.read()[128:]
            self.assertEqual(body, np.array([[1.5, 0.0], [2.25, 3.0]], dtype="<f4").tobytes())

    def test_non_finite_is_refused(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                _npy.write_field(os.path.join(d, "f.npy"), np.array([[np.inf]]))


def _launch(script_text, d):
    script = os.path.join(d, "s.py")
    with open(script, "w") as f:
        f.write(textwrap.dedent(script_text))
    env = dict(os.environ, PYTHONPATH=SDK, PYTHONSAFEPATH="1", PYTHONDONTWRITEBYTECODE="1")
    env.pop("VIA_STEP", None)
    return subprocess.Popen([sys.executable, "-m", "viasdk", script], stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)


def _init(id_=1):
    return _wire.encode({"jsonrpc": "2.0", "id": id_, "method": "initialize",
                         "params": {"protocol": viasdk.PROTOCOL}})


def _step_call(id_, when, d):
    return _wire.encode({"jsonrpc": "2.0", "id": id_, "method": "step.call",
                         "params": {"step": "s", "when": when, "out": d}})


class Launcher(unittest.TestCase):
    def test_print_at_import_never_reaches_the_wire(self):
        with tempfile.TemporaryDirectory() as d:
            p = _launch("""
                print("loud import")
                def lint(step):
                    pass
            """, d)
            p.stdin.write(_init())
            p.stdin.write(_step_call(2, "lint", d))
            p.stdin.flush()
            # Read the answers before quit: input that ends while a call is queued ends the
            # worker without serving it (_wire).
            init = json.loads(_wire.read_frame(p.stdout))
            call = json.loads(_wire.read_frame(p.stdout))
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "id": 3, "method": "shutdown"}))
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "method": "quit"}))
            p.stdin.flush()
            out, err = p.communicate(timeout=20)
            self.assertEqual(p.returncode, 0)
            self.assertEqual(init["result"]["protocol"], "via-step-1")
            # All three points, though the script defines only lint (protocol.md 3.2).
            self.assertEqual(init["result"]["capabilities"]["when"], ["generate", "lint", "export"])
            self.assertEqual(init["result"]["sdk"]["version"], viasdk.__version__)
            self.assertEqual(call, {"jsonrpc": "2.0", "id": 2, "result": None})
            r = io.BytesIO(out)
            self.assertEqual(json.loads(_wire.read_frame(r)), {"jsonrpc": "2.0", "id": 3, "result": None})
            self.assertIsNone(_wire.read_frame(r), "nothing but frames on the wire")
            self.assertIn(b"loud import", err)

    def test_initialize_is_answered_before_the_import(self):
        # protocol.md 3.1 and 8: descriptors, initialize, then the import. A slow import does not
        # count against initialize's deadline (6.2).
        with tempfile.TemporaryDirectory() as d:
            p = _launch("""
                import time
                time.sleep(3)
                def lint(step):
                    pass
            """, d)
            t0 = time.monotonic()
            p.stdin.write(_init())
            p.stdin.flush()
            init = json.loads(_wire.read_frame(p.stdout))
            self.assertLess(time.monotonic() - t0, 2.5, "initialize waited for the import")
            self.assertEqual(init["result"]["protocol"], "via-step-1")
            p.stdin.write(_step_call(2, "lint", d))
            p.stdin.flush()
            call = json.loads(_wire.read_frame(p.stdout))
            self.assertEqual(call["result"], None)
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "method": "quit"}))
            p.stdin.flush()
            p.communicate(timeout=20)

    def test_an_import_error_is_raised_at_step_call(self):
        # 3.4: -32010 answers step.call; initialize succeeds and knows nothing of the script.
        with tempfile.TemporaryDirectory() as d:
            p = _launch("raise ImportError('no such library')\n", d)
            p.stdin.write(_init())
            p.stdin.write(_step_call(2, "generate", d))
            p.stdin.flush()
            init = json.loads(_wire.read_frame(p.stdout))
            call = json.loads(_wire.read_frame(p.stdout))
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "method": "quit"}))
            p.stdin.flush()
            _, err = p.communicate(timeout=20)
            self.assertIn("result", init)
            self.assertEqual(call["error"]["code"], -32010)
            self.assertEqual(call["error"]["data"]["exception"], "ImportError")
            self.assertIn(b"no such library", err)

    def test_another_version_is_refused_and_exits(self):
        with tempfile.TemporaryDirectory() as d:
            p = _launch("def generate(step):\n    pass\n", d)
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                        "params": {"protocol": "via-step-2"}}))
            p.stdin.flush()
            out, _ = p.communicate(timeout=20)
            self.assertEqual(p.returncode, 1)
            msg = json.loads(_wire.read_frame(io.BytesIO(out)))
            self.assertEqual(msg["error"]["code"], -32001)
            self.assertEqual(msg["error"]["data"]["protocol"], "via-step-1")

    def test_missing_function_is_no_entry_at_step_call(self):
        # initialize lists all three points; the missing function surfaces at step.call as
        # protocol.no_entry, the one code for it (protocol.md 3.2).
        with tempfile.TemporaryDirectory() as d:
            p = _launch("def lint(step):\n    pass\n", d)
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                        "params": {"protocol": viasdk.PROTOCOL}}))
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "id": 2, "method": "step.call",
                                        "params": {"when": "generate"}}))
            p.stdin.flush()
            # Read both answers before quit: input that ends while a call is queued ends the
            # worker without serving it (_wire).
            init = json.loads(_wire.read_frame(p.stdout))
            self.assertIn("generate", init["result"]["capabilities"]["when"])
            call = json.loads(_wire.read_frame(p.stdout))
            p.stdin.write(_wire.encode({"jsonrpc": "2.0", "method": "quit"}))
            p.stdin.flush()
            p.communicate(timeout=20)
            self.assertEqual(p.returncode, 0)
            self.assertEqual(call["id"], 2)
            self.assertEqual(call["error"]["code"], -32601)
            self.assertEqual(call["error"]["data"]["code"], "protocol.no_entry")

    def test_input_ending_ends_the_worker(self):
        with tempfile.TemporaryDirectory() as d:
            p = _launch("def generate(step):\n    pass\n", d)
            p.stdin.close()
            self.assertEqual(p.wait(timeout=20), 0)


class Version(unittest.TestCase):
    def test_major_version_is_the_protocol_number(self):
        self.assertEqual(viasdk.PROTOCOL, "via-step-1")
        self.assertEqual(viasdk.__version__.split(".")[0], "1")


if __name__ == "__main__":
    unittest.main()
