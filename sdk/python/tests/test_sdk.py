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


class PlainCommand(unittest.TestCase):
    """protocol.md 2.2 and 8: ``python -m viasdk <script>`` with VIA_STEP set."""

    def _run(self, script_text, data, when="lint"):
        d = tempfile.mkdtemp()
        objects = os.path.join(d, "objects")
        out = os.path.join(d, "out")
        os.makedirs(objects)
        os.makedirs(out)
        field = os.path.join(d, "h.npy")
        _npy.write_field(field, np.array([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]))
        data.setdefault("fields", {"height": {
            "type": "scalar", "role": "surface", "tags": ["ground"], "unit": "m",
            "blob": "x", "path": field, "i0": 0, "j0": 0, "i1": 3, "j1": 2}})
        req = {"protocol": "via-step-1", "step": "checks", "when": when, "for_each_item": None,
               "params": {}, "grid": {"extent_mm": [0, 0, 2000, 1000], "resolution_mm": 1000,
                                      "nx": 3, "ny": 2},
               "seed": "0" * 16, "out": out, "objects": objects, "data": data}
        rp = os.path.join(d, "request.json")
        with open(rp, "w") as f:
            json.dump(req, f)
        sp = os.path.join(d, "s.py")
        with open(sp, "w") as f:
            f.write(textwrap.dedent(script_text))
        env = dict(os.environ, VIA_STEP=rp, PYTHONPATH=SDK)
        p = subprocess.run([sys.executable, "-m", "viasdk", sp], env=env, capture_output=True,
                           text=True, timeout=60, stdin=subprocess.DEVNULL)
        return p, out

    def test_lint_reads_by_role_and_tag_and_writes_diagnostics(self):
        data = {
            "features": {
                "name": {"dtype": "str", "values": ["a", "b"]},
                "kind": {"dtype": "str", "values": ["lane", "ford"]},
                "geometry": {"dtype": "u8", "values": [1, 0]},
                "pieces": {"dtype": "i64", "values": [0, 1, 2]},
                "rings": {"dtype": "i64", "values": [0, 1, 2]},
                "ring_starts": {"dtype": "i64", "values": [0, 2, 3]},
                "coords": {"dtype": "i32", "shape": [3, 2],
                           "values": [[0, 0], [1000, 0], [500, 500]]},
            },
            "instances": {
                "name": {"dtype": "str", "values": ["t#1", "house"]},
                "by": {"dtype": "str", "values": ["t", "instance.house"]},
                "taxon": {"dtype": "str", "values": ["plant/tree", "building/house"]},
                "x": {"dtype": "i32", "values": [1, 2]},
            },
        }
        p, out = self._run("""
            def lint(step):
                (h,) = step.fields(["role:surface"])
                (g,) = step.fields(["tag:ground"])
                f = step.features("kind:ford")
                t = step.instances("@plant")
                step.info(code="m", subject="feature.b", message="m", hint="h",
                          data={"v": float(h[1][2] + g[0][0]), "n": len(f["name"]),
                                "c": f["coords"].tolist(), "t": list(t["name"])},
                          explain="e")
        """, data)
        self.assertEqual(p.returncode, 0, p.stderr)
        with open(os.path.join(out, "diagnostics.json")) as f:
            recs = json.load(f)
        self.assertEqual(recs[0]["data"], {"v": 7.0, "n": 1, "c": [[500, 500]], "t": ["t#1"]})
        self.assertEqual(recs[0]["explain"], "e")

    def test_window_halo_and_within_are_honoured(self):
        # protocol.md 8: the same script runs as a worker or a plain command, and gets the same
        # posts and the same rows either way.
        data = {
            "features": {
                "name": {"dtype": "str", "values": ["a", "b", "zone"]},
                "kind": {"dtype": "str", "values": ["lane", "ford", "area"]},
                "geometry": {"dtype": "u8", "values": [0, 0, 2]},
                "pieces": {"dtype": "i64", "values": [0, 1, 2, 3]},
                "rings": {"dtype": "i64", "values": [0, 1, 2, 3]},
                "ring_starts": {"dtype": "i64", "values": [0, 1, 2, 6]},
                "coords": {"dtype": "i32", "shape": [6, 2],
                           "values": [[100, 100], [1500, 900], [0, 0], [1000, 0], [1000, 1000],
                                      [0, 1000]]},
            },
            "instances": {
                "name": {"dtype": "str", "values": ["in", "out"]},
                "by": {"dtype": "str", "values": ["t", "t"]},
                "x": {"dtype": "i32", "values": [500, 1800]},
                "y": {"dtype": "i32", "values": [500, 500]},
            },
        }
        p, out = self._run("""
            def lint(step):
                (w,) = step.fields(["height"], window=[1, 1, 2, 2], halo=1)
                f = step.features("*", within="zone")
                g = step.features("*", within=[[1200, 0], [2000, 0], [2000, 1000], [1200, 1000]])
                t = step.instances("*", within="zone")
                step.info(code="m", subject="feature.b", message="m", hint="h",
                          data={"w": w.tolist(), "win": list(w.window), "f": list(f["name"]),
                                "g": list(g["name"]), "t": list(t["name"])})
        """, data)
        self.assertEqual(p.returncode, 0, p.stderr)
        with open(os.path.join(out, "diagnostics.json")) as f:
            recs = json.load(f)
        self.assertEqual(recs[0]["data"], {"w": [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
                                           "win": [0, 0, 3, 2], "f": ["a", "zone"], "g": ["b"],
                                           "t": ["in"]})
        p, out = self._run("""
            def lint(step):
                (w,) = step.fields(["height"], window=[2, 0, 3, 1])
                step.info(code="m", subject="feature.b", message="m", hint="h",
                          data={"w": w.tolist()})
        """, {})
        with open(os.path.join(out, "diagnostics.json")) as f:
            self.assertEqual(json.load(f)[0]["data"], {"w": [[3.0]]})

    def test_a_view_it_was_not_handed_is_refused(self):
        p, _ = self._run("""
            def generate(step):
                step.fields(["height"], view="head")
        """, {}, when="generate")
        self.assertEqual(p.returncode, 1)
        self.assertIn("view='head'", p.stderr)

    def test_missing_function_and_exception_exit_1(self):
        p, _ = self._run("x = 1\n", {})
        self.assertEqual(p.returncode, 1)
        self.assertIn("lint(step)", p.stderr)
        p, _ = self._run("def lint(step):\n    raise ValueError('boom')\n", {})
        self.assertEqual(p.returncode, 1)
        self.assertIn("boom", p.stderr)
        p, _ = self._run("def lint(step):\n    step.fields(['role:none'])\n", {})
        self.assertEqual(p.returncode, 1)
        self.assertIn("LookupError", p.stderr)


class Blake3(unittest.TestCase):
    """The official test vectors' input (byte i is i mod 251), checked at the lengths where the
    chunk tree changes shape; digests from the reference implementation."""

    VECTORS = {
        0: "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        1: "2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213",
        1023: "10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11",
        1024: "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7",
        1025: "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444",
        2049: "5f4d72f40d7a5f82b15ca2b2e44b1de3c2ef86c426c95c1af0b6879522563030",
        2048: "e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a",
        3072: "b98cb0ff3623be03326b373de6b9095218513e64f1ee2edd2525c7ad1e5cffd2",
        3073: "7124b49501012f81cc7f11ca069ec9226cecb8a2c850cfe644e327d22d3e1cd3",
    }

    def test_vectors_at_tree_shapes(self):
        from viasdk import _blake3

        for n, want in self.VECTORS.items():
            self.assertEqual(_blake3.digest(bytes(i % 251 for i in range(n))), want, n)

    def test_a_blob_is_named_by_its_digest(self):
        from viasdk import _blake3, _columns

        with tempfile.TemporaryDirectory() as d:
            data = bytes(i % 251 for i in range(9000))
            name = _columns.put_blob(d, data)
            self.assertEqual(name, _blake3.digest(data))
            with open(os.path.join(d, name[:2], name), "rb") as f:
                self.assertEqual(f.read(), data)
            self.assertEqual(_columns.put_blob(d, data), name)  # kept, never replaced


class Columns(unittest.TestCase):
    def test_round_trip_inline_and_blob(self):
        from viasdk import _columns

        with tempfile.TemporaryDirectory() as d:
            for n in (3, 5000):
                a = np.arange(n, dtype=np.int32)
                col = _columns.to_wire(a, "i32", d)
                self.assertEqual("blob" in col, n > 4096)
                self.assertTrue(np.array_equal(_columns.from_wire(col, d), a))
                s = ["é%d" % k for k in range(n)]
                self.assertEqual(_columns.from_wire(_columns.to_wire(s, "str", d), d), s)
            two = _columns.to_wire(np.array([[1, 2], [3, 4]]), "i32", d)
            self.assertEqual(two["shape"], [2, 2])
            f = _columns.to_wire([-0.0, 1.5], "f64")
            self.assertEqual(f["values"], [0.0, 1.5])
            self.assertFalse(str(f["values"][0]).startswith("-"))
            with self.assertRaises(ValueError):
                _columns.to_wire([float("nan")], "f64")


class Document(unittest.TestCase):
    def test_metres_are_exact_from_millimetres(self):
        self.assertEqual(viasdk.metres(1500), "1.5")
        self.assertEqual(viasdk.metres(1), "0.001")
        self.assertEqual(viasdk.metres(-2000), "-2")
        self.assertEqual(viasdk.metres(0), "0")

    def test_out_writes_tables_and_field_files(self):
        from viasdk._step import Out

        with tempfile.TemporaryDirectory() as d:
            out = Out(d)
            out.field("soft", np.zeros((2, 3), dtype=np.float32), tags=["soft"])
            out.row("height", "soften", value="= mix(v, f('soft'), 0.3)")
            out.feature("k", "mark", at=(1500, 2000), width=2.5)
            out.instance("i", "tree_oak", at=(0, 1), face=90)
            text = out.text()
            self.assertIn('[field.soft]\nfile = "soft.npy"\ntags = ["soft"]\noutside = 0\n', text)
            self.assertIn("[field.height.then.soften]\nvalue = \"= mix(v, f('soft'), 0.3)\"", text)
            self.assertIn('[feature.k]\nkind = "mark"\nat = [1.5, 2]\nwidth = 2.5', text)
            self.assertIn('[instance.i]\nasset = "tree_oak"\nat = [0, 0.001]\nface = 90', text)
            self.assertTrue(os.path.exists(os.path.join(d, "soft.npy")))
            import tomllib

            tomllib.loads(text)

    def test_out_writes_a_feature_of_no_kind(self):
        # M7 close (`mill`, the second model family): `feature` always wrote `kind`, so a kindless
        # walk point came back as `kind = ""`, `authority.unknown_kind`.
        from viasdk._step import Out

        with tempfile.TemporaryDirectory() as d:
            out = Out(d)
            out.feature("stair_head", at=(1500, 2000))
            out.feature("walk", None, path=[(0, 0), (1500, 2000)])
            import tomllib

            got = tomllib.loads(out.text())
            self.assertEqual(got["feature"]["stair_head"], {"at": [1.5, 2]})
            self.assertEqual(got["feature"]["walk"], {"path": [[0, 0], [1.5, 2]]})

    def test_out_writes_a_run_and_refuses_other_tables(self):
        from viasdk._step import Out

        with tempfile.TemporaryDirectory() as d:
            out = Out(d)
            out.table("run", "w0", asset="wall", count=3, on={"ref": "hall_s0", "span": [0, 0.5]})
            import tomllib

            got = tomllib.loads(out.text())
            self.assertEqual(got["run"]["w0"], {"asset": "wall", "count": 3,
                                                "on": {"ref": "hall_s0", "span": [0, 0.5]}})
            with self.assertRaises(ValueError):
                out.table("step", "x")


class Meetings(unittest.TestCase):
    """Step.meetings (surfaces.md 2.4): one surface.meetings call, carrying the step.call's id,
    its table decoded; the engine's refusal (surface.too_early at generate) raised to the step."""

    class Conn:
        def __init__(self, answer=None, error=None):
            self.cancelled, self.sent, self.answer, self.error = set(), [], answer, error

        def request(self, method, params):
            self.sent.append((method, params))
            if self.error is not None:
                raise self.error
            return self.answer

    def step(self, conn, when="lint"):
        from viasdk._step import Step

        return Step({"step": "s", "when": when, "out": tempfile.mkdtemp()}, conn, 7)

    def test_one_call_and_its_table(self):
        conn = self.Conn({"a": {"dtype": "str", "values": ["m.end"]},
                          "b": {"dtype": "str", "values": ["n.start"]},
                          "gap": {"dtype": "i64", "values": [0]}})
        t = self.step(conn).meetings()
        self.assertEqual(conn.sent, [("surface.meetings", {"call": 7})])
        self.assertEqual(list(t["a"]), ["m.end"])
        self.assertEqual(list(t["gap"]), [0])

    def test_a_refusal_is_raised(self):
        err = viasdk.RpcError(-32006, "too early", {"code": "surface.too_early"})
        with self.assertRaises(viasdk.RpcError):
            self.step(self.Conn(error=err), when="generate").meetings()


class SurfaceQuestions(Meetings):
    """Step.surfaces_at and Step.sample (surfaces.md 2.1, 2.2): one batch call each, columns in
    (x, y as i32 millimetres) and a table out (R23), the step.call's id carried."""

    def test_surfaces_at_is_one_batch(self):
        conn = self.Conn({"point": {"dtype": "i64", "values": [0, 0, 1]},
                          "surface": {"dtype": "str", "values": ["height", "s.floor", "height"]},
                          "z": {"dtype": "f64", "values": [1.0, 3.0, 2.5]}})
        t = self.step(conn).surfaces_at([0, 1000], [0, 500])
        ((method, params),) = conn.sent
        self.assertEqual(method, "surface.surfaces_at")
        self.assertEqual(params["call"], 7)
        self.assertEqual(params["view"], "full")
        self.assertEqual((params["x"]["dtype"], list(params["x"]["values"])), ("i32", [0, 1000]))
        self.assertEqual((params["y"]["dtype"], list(params["y"]["values"])), ("i32", [0, 500]))
        self.assertEqual(list(t["surface"]), ["height", "s.floor", "height"])
        self.assertEqual(list(t["point"]), [0, 0, 1])

    def test_sample_names_one_surface(self):
        conn = self.Conn({"inside": {"dtype": "bool", "values": [1, 0]},
                          "z": {"dtype": "f64", "values": [3.0, 0.0]},
                          "gx": {"dtype": "f64", "values": [0.5, 0.0]},
                          "gy": {"dtype": "f64", "values": [0.0, 0.0]}})
        t = self.step(conn, when="generate").sample("s.floor", [0, 9000], [0, 0], view="head")
        ((method, params),) = conn.sent
        self.assertEqual((method, params["surface"], params["view"]), ("surface.sample", "s.floor", "head"))
        self.assertEqual([bool(v) for v in t["inside"]], [True, False])
        self.assertEqual(list(t["gx"]), [0.5, 0.0])

    def test_a_refusal_is_raised(self):
        err = viasdk.RpcError(-32602, "no surface", {"code": "surface.not_surface"})
        with self.assertRaises(viasdk.RpcError):
            self.step(self.Conn(error=err)).sample("nowhere", [0], [0])


class Version(unittest.TestCase):
    def test_major_version_is_the_protocol_number(self):
        self.assertEqual(viasdk.PROTOCOL, "via-step-1")
        self.assertEqual(viasdk.__version__.split(".")[0], "1")


class KitReads(unittest.TestCase):
    """viasdk.kit, attrs and ground (the fix plan's F4): what six libraries each wrote."""

    class Fake:
        def __init__(self, surface=True):
            self.surface, self.asked = surface, []

        def pack(self, select, columns=()):
            self.asked.append((select, list(columns)))
            return {"id": ["bay", "post"], "tags": ['["role:wall"]', '["role:post", "storey:ground"]'],
                    "bounds": [[-1500, -150, 0, 1500, 150, 3000], [-150, -150, 0, 150, 150, 3000]],
                    "params": ['{"pitch": 3.0}', ""], "shared": ['{"module": 3.0}', '{"module": 3.0}'],
                    "sink": [0.2, 0.0]}

        def fields(self, names, **kw):
            if not self.surface:
                raise LookupError("no surface")

            class F(list):
                name = "height"
            return [F()]

        def eval(self, formula, x, y):
            assert formula == "= f('height')"
            return [10.0 + v / 1000 for v in x]

    def test_a_kit_is_its_pieces_decoded(self):
        from viasdk import kit
        s = self.Fake()
        rows = kit(s, "structure/plain")
        self.assertEqual(s.asked, [("@structure/plain", ["bounds", "params", "shared"])])
        self.assertEqual(rows[0], {"id": "bay", "taxon": "structure/plain", "tags": ["role:wall"],
                                   "bounds": [-1500, -150, 0, 1500, 150, 3000], "params": {"pitch": 3.0},
                                   "shared": {"module": 3.0}})
        self.assertEqual(rows[1]["params"], {})
        rows = kit(s, "x", columns=("sink", "footprint"))
        self.assertEqual((rows[0]["sink"], rows[0]["footprint"]), (0.2, []))

    def test_attributes_past_their_null_columns(self):
        from viasdk import attrs
        fl = {"name": ["a", "b"], "geometry": [3, 3], "storeys": [2, 0], "storeys?": [1, 0],
              "doors": ['[{"side": 0}]', ""], "kit": ["k", "k"]}
        self.assertEqual(attrs(fl, 0), {"storeys": 2, "doors": [{"side": 0}], "kit": "k"})
        self.assertEqual(attrs(fl, 1, ["storeys", "kit"]), {"kit": "k"})

    def test_the_ground_under_points(self):
        from viasdk import ground
        self.assertEqual(ground(self.Fake(), [0, 1000], [0, 0]), [10.0, 11.0])
        self.assertEqual(ground(self.Fake(), [], []), [])
        self.assertIsNone(ground(self.Fake(surface=False), [0], [0]))

    def test_the_head_view_is_asked_of_the_field_too(self):
        """F4's review: a step that returns rows on the surface reads it in its head view."""
        from viasdk import ground, kit

        class Head(self.Fake):
            def fields(self, names, **kw):
                self.asked.append(("fields", kw.get("view")))
                return super().fields(names, **kw)

            def eval(self, formula, x, y, view="full"):
                self.asked.append(("eval", view))
                return super().eval(formula, x, y)
        s = Head()
        ground(s, [0], [0], view="head")
        self.assertEqual(s.asked, [("fields", "head"), ("eval", "head")])
        self.assertEqual(kit(self.Fake(), "x", columns="bounds")[0]["bounds"], [-1500, -150, 0, 1500, 150, 3000])

    def test_the_methods_are_the_functions(self):
        from viasdk import FeatureList, Step
        fl = FeatureList({"name": ["a"], "geometry": [3], "class": ["x"], "kit": ["k"]})
        self.assertEqual(fl.attrs(0), {"kit": "k"})
        for m in ("kit", "ground"):
            self.assertTrue(callable(getattr(Step, m)))


if __name__ == "__main__":
    unittest.main()
