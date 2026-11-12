"""Self-tests of the conformance runner: it is driven against tests/fake_via.py, a fake binary that
is honest by default and breaks one rule on request, so each check is seen to pass and to fail.

Run with `uv run python -m unittest discover -s tests` from tools/conformance/.
"""

from __future__ import annotations

import io
import json
import shutil
import sys
import tempfile
import textwrap
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from via_conformance import canon, cli, diags, fixtures, leaks, model, record  # noqa: E402
from via_conformance.runner import Settings, _pid_alive, run_case  # noqa: E402


def write(path: Path, text: str) -> None:
    """UTF-8 and `\n` on every system: `write_text` alone gives the locale's code page and `\r\n`
    on Windows."""
    path.write_bytes(text.encode("utf-8"))

REGISTRY = textwrap.dedent(
    """\
    [cli.usage]
    severity = "error"
    [identity.no_previous]
    severity = "info"
    [formula.unknown_name]
    severity = "error"
    [test.warn]
    severity = "warning"
    [test.note]
    severity = "info"
    [check.digest]
    severity = "error"
    """
)

WORLD = textwrap.dedent(
    """\
    name = "t"

    [field.height]
    value = 1.5

    [field.slope]
    value = 0.25

    [feature.mark]
    at = [1, 2]

    [instance.hut]
    at = [3, 4]

    [instance.shed]
    at = [10, 10]
    """
)


class Base(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="via-conformance-test-"))
        self.repo = self.tmp / "repo"
        (self.repo / "spec" / "cases").mkdir(parents=True)
        (self.repo / "lib").mkdir()
        for ch in ("build", "graph"):
            write(self.repo / "spec" / f"{ch}.md", f"# {ch}\n")
        write(self.repo / "spec" / "diagnostics.toml", REGISTRY)
        (self.repo / "examples" / "w").mkdir(parents=True)
        write(self.repo / "examples" / "w" / "world.toml", WORLD)
        # A `.py` binary runs under this Python (Settings.argv), on Windows as elsewhere.
        self.via = HERE / "fake_via.py"
        self.work_root = self.tmp / "work"
        self.work_root.mkdir()

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def case(self, text: str, files: dict[str, str | bytes] | None = None, chapter: str = "build",
             cid: str | None = None) -> Path:
        if cid is None:
            self.serial = getattr(self, "serial", 0) + 1
            cid = f"c{self.serial}"
        folder = self.repo / "spec" / "cases" / chapter / cid
        folder.mkdir(parents=True)
        write(folder / "case.toml", textwrap.dedent(text))
        for rel, data in (files or {}).items():
            p = folder / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            if isinstance(data, bytes):
                p.write_bytes(data)
            else:
                write(p, data)
        return folder

    def run_folder(self, folder: Path, milestone: int = 1, timeout: float = 60):
        c = model.parse_case(folder, folder.parent.name)
        settings = Settings(via=self.via, repo=self.repo, milestone=milestone, work_root=self.work_root, timeout=timeout)
        return run_case(c, settings, diags.Registry(self.repo / "spec" / "diagnostics.toml"))

    def assertPass(self, outcome) -> None:
        self.assertEqual(outcome.status, "pass", "\n".join(outcome.detail))

    def assertFailsWith(self, outcome, check: str, text: str = "") -> None:
        self.assertEqual(outcome.status, "fail", "\n".join(outcome.detail))
        joined = "\n".join(outcome.detail)
        self.assertIn(f"check {check}", joined)
        if text:
            self.assertIn(text, joined)


HEAD = """\
format    = "via-case-1"
summary   = "A test."
milestone = 1
"""

BUILD = HEAD + """\
actions   = ["build"]

[copy.w]
from = "examples/w"
to   = "."

[action.build]
via  = ["build", "world.toml", "--out", "build/t"]
out  = "build/t"
exit = 0
"""


class CanonTests(unittest.TestCase):
    def test_ecmascript_numbers(self) -> None:
        cases = {3.0: "3", 0.1: "0.1", 1e21: "1e+21", 1e-7: "1e-7", 5e-324: "5e-324", 1.23e-18: "1.23e-18",
                 123456789012345680000.0: "123456789012345680000", 1.5: "1.5", -0.001: "-0.001",
                 1e-6: "0.000001", 31.8: "31.8", 100.0: "100", 2.5e20: "250000000000000000000"}
        for x, want in cases.items():
            self.assertEqual(canon.encode(x), want, x)

    def test_canonical(self) -> None:
        self.assertTrue(canon.is_canonical(b'{"a":1,"b":[true,null,"x"]}\n'))
        self.assertFalse(canon.is_canonical(b'{"b":1,"a":2}\n'))
        self.assertFalse(canon.is_canonical(b'{"a": 1}\n'))
        self.assertFalse(canon.is_canonical(b'{"a":1}'))
        self.assertFalse(canon.is_canonical(b'{"a":3.0}\n'))
        self.assertFalse(canon.is_canonical(b'{"a":-0}\n'))
        self.assertTrue(canon.is_canonical('{"z":"\\u0001","\u00e9":"\u00e9"}\n'.encode()))
        self.assertFalse(canon.is_canonical('{"\u00e9":1,"z":2}\n'.encode()))

    def test_pointer_and_values(self) -> None:
        d = {"a": [{"b~/c": 5}], "": 1}
        self.assertEqual(canon.resolve(d, "/a/0/b~0~1c"), 5)
        with self.assertRaises(canon.PointerError):
            canon.resolve(d, "/a/1")
        self.assertTrue(canon.values_equal(31.8, 31.8))
        self.assertTrue(canon.values_equal(3, 3.0))
        self.assertFalse(canon.values_equal(True, 1))
        self.assertFalse(canon.values_equal({"a": 1}, {"a": 1, "b": 2}))
        self.assertTrue(canon.includes({"a": 1}, {"a": 1, "b": 2}))
        self.assertTrue(canon.includes([2, 2], [2, 1, 2]))
        self.assertFalse(canon.includes([2, 2], [2, 1]))


class ParseTests(Base):
    def expect_error(self, text: str, fragment: str) -> None:
        folder = self.case(text, cid=f"e{len(list((self.repo / 'spec' / 'cases').rglob('case.toml')))}")
        with self.assertRaises(model.CaseError) as ctx:
            model.parse_case(folder, "build")
        self.assertIn(fragment, str(ctx.exception))

    def test_errors(self) -> None:
        self.expect_error(BUILD + "colour = 1\n", "unknown key 'colour'")
        self.expect_error(BUILD.replace('summary   = "A test."\n', ""), "missing required key 'summary'")
        self.expect_error(BUILD.replace("milestone = 1", 'milestone = "1"'), "expected an integer")
        self.expect_error(BUILD.replace("milestone = 1", "milestone = 9"), "above 7")
        self.expect_error(BUILD.replace('actions   = ["build"]', 'actions = ["build", "build"]'), "listed twice")
        self.expect_error(BUILD + '[action.x]\nvia = ["help"]\nexit = 0\n', "not listed in actions")
        self.expect_error(BUILD.replace('actions   = ["build"]', 'actions = ["nope"]'), "names no [action.nope]")
        self.expect_error(BUILD.replace('out  = "build/t"\n', ""), "`out` is required")
        self.expect_error(BUILD + '[action.build.file.f]\nbogus = "x"\n', "unknown key 'bogus'")
        self.expect_error(BUILD + '[action.build.check.k]\nfn = "sum_of_all"\n', "not a named check")
        self.expect_error(BUILD + '[action.build.check.k]\nfn = "weights_sum_at_most"\npath = "a"\n',
                          "missing required key 'limit'")
        self.expect_error(BUILD + '[action.build.same.s]\naction = "build"\nwhat = "id"\n', "not an earlier via action")
        self.expect_error(BUILD.replace('from = "examples/w"', 'from = "spike/worlds"'), "never copies from spike/")
        self.expect_error(BUILD.replace('"--out", "build/t"', '"--diagnostics", "x"'), "passes --diagnostics itself")
        self.expect_error(BUILD + '[action.build.file.f]\npath = "/etc/passwd"\nexists = true\n', "must be relative")
        self.expect_error(BUILD.replace('format    = "via-case-1"', 'format = "via-case-2"'), "is not 'via-case-1'")
        self.expect_error(BUILD + '[action.build.diag.d]\ncode = "a.b"\nfrom = "both"\n', "is not world or run")

    def test_bad_case_id(self) -> None:
        folder = self.case(BUILD, cid="Bad")
        with self.assertRaises(model.CaseError):
            model.parse_case(folder, "build")


class RunTests(Base):
    def test_version_help(self) -> None:
        o = self.run_folder(self.case(HEAD + """\
            actions = ["version", "help"]
            [action.version]
            via  = ["--version"]
            exit = 0
            [action.version.file.out]
            stream = "stdout"
            text   = "via 0.1.0\\n"
            [action.help]
            via  = ["help"]
            exit = 0
            """))
        self.assertPass(o)

    def test_build_checks_pass(self) -> None:
        o = self.run_folder(self.case(BUILD.replace('actions   = ["build"]', 'actions = ["build", "again", "usage"]') + """\
            [action.build.diag.first]
            code = "identity.no_previous"
            from = "run"
            [action.build.file.listing]
            path    = "build/t"
            listing = [".previous", "changes.json", "diagnostics.json", "features.json", "fields", "instances", "manifest.json"]
            [action.build.file.world]
            path    = "build/t/manifest.json"
            pointer = "/world"
            value   = "t"
            [action.build.file.files]
            path     = "build/t/manifest.json"
            pointer  = "/files/fields~1height.npy"
            includes = { bytes = 164 }
            [action.build.file.height]
            path  = "build/t/fields/height.npy"
            dtype = "<f4"
            shape = [3, 3]
            all   = 1.5
            [action.build.file.post]
            path  = "build/t/fields/slope.npy"
            post  = [2, 1]
            bits  = "0x3E800000"
            [action.build.file.post_value]
            path  = "build/t/fields/slope.npy"
            post  = [0, 2]
            value = 0.25
            [action.build.file.features]
            path   = "build/t/features.json"
            equals = "expected/features.json"
            ignore = ["/footprints"]
            [action.build.nodes]
            ran = ["field:*", "merge"]
            [action.build.check.hut]
            fn     = "no_origin_inside"
            build  = "build/t"
            at     = [20, 20]
            radius = 5
            [action.build.check.square]
            fn     = "no_origin_inside"
            build  = "build/t"
            points = [[5, 5], [6, 5], [6, 6], [5, 6]]
            [action.again]
            via       = ["build", "world.toml", "--out", "build/t"]
            out       = "build/t"
            exit      = 0
            untouched = ["build/t"]
            [action.again.nodes]
            ran    = []
            reused = ["field:*"]
            [action.again.same.id]
            action = "build"
            what   = "id"
            [action.again.same.files]
            action = "build"
            what   = "files"
            [action.again.same.diag]
            action = "build"
            what   = "diagnostics"
            [action.again.same.feat]
            action = "build"
            what   = "build/t/features.json"
            [action.usage]
            via  = ["frobnicate"]
            exit = 2
            [action.usage.diag.u]
            code     = "cli.usage"
            argument = 1
            """, files={"expected/features.json":
                        '{"features":[{"at":[1,2,0],"kind":null,"name":"mark"}],"footprints":["x"]}\n'}))
        self.assertPass(o)

    def test_edits_and_for_each(self) -> None:
        o = self.run_folder(self.case(BUILD.replace('actions   = ["build"]', 'actions = ["edit", "add", "flip", "cut", "gone", "build", "each"]') + """\
            [action.edit]
            replace = "world.toml"
            find    = "value = 1.5"
            with    = "value = 2"
            [action.add]
            write = "notes/a.txt"
            from  = "later/a.txt"
            [action.flip]
            flip = "notes/a.txt"
            at   = 0
            [action.cut]
            truncate = "notes/a.txt"
            bytes    = 1
            [action.gone]
            delete = "notes"
            [action.build.file.height]
            path = "build/t/fields/height.npy"
            all  = 2
            [action.build.file.notes]
            path   = "notes"
            exists = false
            [action.each]
            via      = ["check", "{item}"]
            for_each = "build/*/"
            exit     = 0
            """, files={"later/a.txt": "abc"}))
        self.assertPass(o)

    def test_change_report_checks(self) -> None:
        o = self.run_folder(self.case(BUILD.replace('actions   = ["build"]', 'actions = ["build", "edit", "second"]') + """\
            [action.edit]
            replace = "world.toml"
            find    = "value = 1.5"
            with    = "value = 3"
            [action.second]
            via  = ["build", "world.toml", "--out", "build/u", "--against", "build/t"]
            out  = "build/u"
            exit = 0
            [action.second.check.fields]
            fn     = "changes_by_prefix"
            path   = "build/u/changes.json"
            prefix = "fields/h"
            what   = ["changed"]
            [action.second.same.id]
            action = "build"
            what   = "id"
            differ = true
            """))
        self.assertPass(o)

    def test_pending(self) -> None:
        above = self.run_folder(self.case(BUILD.replace("milestone = 1", "milestone = 3"), cid="above"))
        self.assertEqual((above.status, above.detail), ("pending", ["M3"]))
        reason = self.run_folder(self.case(HEAD + 'pending = "fixture"\nactions = ["v"]\n[action.v]\nvia = ["--version"]\nexit = 0\n', cid="reason"))
        self.assertEqual((reason.status, reason.detail), ("pending", ["fixture"]))


class FailTests(Base):
    """Every rule the runner adds, seen failing."""

    def build_with(self, extra: str = "", env: str = "", head: str = BUILD, files=None):
        text = head
        if env:
            text = text.replace('exit = 0\n', f'exit = 0\nenv  = {{ FAKE_VIA = "{env}" }}\n', 1)
        return self.run_folder(self.case(text + extra, files=files))

    def test_u1_exit(self) -> None:
        o = self.run_folder(self.case(HEAD + 'actions = ["v"]\n[action.v]\nvia = ["--version"]\nexit = 1\n'))
        self.assertFailsWith(o, "U1", "exit 0")

    def test_diag_document_missing(self) -> None:
        self.assertFailsWith(self.build_with(env="nodiag"), "3.5", "no file")

    def test_diag_status(self) -> None:
        self.assertFailsWith(self.build_with(env="status"), "3.5.status")

    def test_u2_unregistered_and_no_hint(self) -> None:
        world = WORLD + '[[fake.diag]]\ncode = "test.unknown"\nseverity = "info"\nsubject = "world"\nhint = "x"\nfile = "world.toml"\n' \
                        '[[fake.diag]]\ncode = "test.note"\nseverity = "info"\nsubject = "world"\nfile = "world.toml"\n' \
                        '[[fake.diag]]\ncode = "test.warn"\nseverity = "info"\nsubject = "world"\nhint = "x"\nfile = "world.toml"\n'
        o = self.build_with(head=BUILD.replace('[copy.w]\nfrom = "examples/w"\nto   = "."\n', ""),
                            extra='diagnostics = "any"\n', files={"world.toml": world})
        joined = "\n".join(o.detail)
        self.assertIn("'test.unknown' is not in diagnostics.toml", joined)
        self.assertIn("empty or missing hint", joined)
        self.assertIn("severity info, the registry says warning", joined)

    def test_u2_failed_on_not_asked(self) -> None:
        world = WORLD + '[[fake.diag]]\ncode = "test.warn"\nseverity = "warning"\nsubject = "world"\nhint = "x"\nfile = "world.toml"\nfailed_on = true\n'
        o = self.build_with(head=BUILD.replace('[copy.w]\nfrom = "examples/w"\nto   = "."\n', ""),
                            extra='[action.build.diag.w]\ncode = "test.warn"\n', files={"world.toml": world})
        self.assertFailsWith(o, "U2", "failed_on on a code --fail-on did not name")

    def test_u3_exit_two_changes_nothing(self) -> None:
        o = self.run_folder(self.case(HEAD + 'actions = ["x"]\n[action.x]\nvia = ["frobnicate"]\nexit = 2\nenv = { FAKE_VIA = "touch2" }\n'
                                      '[action.x.diag.u]\ncode = "cli.usage"\n'))
        self.assertFailsWith(o, "U3", "created oops.txt")

    def test_u4_check(self) -> None:
        self.assertFailsWith(self.build_with(env="checkfail"), "U4")

    def test_u5_absolute(self) -> None:
        self.assertFailsWith(self.build_with(env="abspath"), "U5", "features.json")

    def test_u6_threads(self) -> None:
        self.assertFailsWith(self.build_with(env="threads"), "U6", "1 thread differs from 8")

    def test_u6_incremental(self) -> None:
        text = BUILD.replace('actions   = ["build"]', 'actions = ["build", "edit", "again"]') + """\
            [action.edit]
            replace = "world.toml"
            find    = "value = 1.5"
            with    = "value = 2"
            [action.again]
            via  = ["build", "world.toml", "--out", "build/t"]
            out  = "build/t"
            exit = 0
            env  = { FAKE_VIA = "incremental" }
            """
        self.assertFailsWith(self.run_folder(self.case(text)), "U6", "clean differs from incremental")

    def test_no_rerun_skips_u6(self) -> None:
        text = BUILD.replace('exit = 0\n', 'exit = 0\nenv  = { FAKE_VIA = "threads" }\nno_rerun = "tests threads itself"\n', 1)
        self.assertPass(self.run_folder(self.case(text)))

    def test_exact_leftover(self) -> None:
        world = WORLD + '[[fake.diag]]\ncode = "test.warn"\nseverity = "warning"\nsubject = "world"\nhint = "x"\nfile = "world.toml"\n'
        o = self.build_with(head=BUILD.replace('[copy.w]\nfrom = "examples/w"\nto   = "."\n', ""), files={"world.toml": world})
        self.assertFailsWith(o, "diag", "no world diagnostic left over")
        o2 = self.build_with(head=BUILD.replace('[copy.w]\nfrom = "examples/w"\nto   = "."\n', "")
                             .replace('exit = 0\n', 'exit = 0\ndiagnostics = "includes"\n'), files={"world.toml": world})
        self.assertPass(o2)

    def test_expectation_unmet(self) -> None:
        o = self.build_with(extra='[action.build.diag.typo]\ncode = "formula.unknown_name"\nline = 2\n')
        self.assertFailsWith(o, "diag.typo", "0 of 1 matched")

    def test_untouched(self) -> None:
        text = BUILD.replace('actions   = ["build"]', 'actions = ["build", "again"]') + """\
            [action.again]
            via       = ["build", "world.toml", "--out", "build/t"]
            out       = "build/t"
            exit      = 0
            untouched = ["build/t"]
            env       = { FAKE_VIA = "rewrite" }
            """
        self.assertFailsWith(self.run_folder(self.case(text)), "untouched")

    def test_file_value(self) -> None:
        o = self.build_with(extra='[action.build.file.w]\npath = "build/t/manifest.json"\npointer = "/world"\nvalue = "u"\n')
        self.assertFailsWith(o, "file.w.value", '"u"')
        o = self.build_with(extra='[action.build.file.h]\npath = "build/t/fields/height.npy"\npost = [0, 0]\nvalue = 1.25\n')
        self.assertFailsWith(o, "file.h.value")

    def test_named_check_fails(self) -> None:
        o = self.build_with(extra='[action.build.check.hut]\nfn = "no_origin_inside"\nbuild = "build/t"\nat = [3, 4.5]\nradius = 0.5\n')
        self.assertFailsWith(o, "check.hut", "hut")

    def test_case_errors_at_run(self) -> None:
        o = self.run_folder(self.case(BUILD.replace('actions   = ["build"]', 'actions = ["r", "build"]')
                                      + '[action.r]\nreplace = "world.toml"\nfind = "value"\nwith = "v"\n'))
        self.assertEqual(o.status, "error")
        self.assertIn("occurs 2 times", "\n".join(o.detail))

    def test_u1_timeout_kills_the_tree(self) -> None:
        # The worker inherits via's stdout and stderr; the runner must neither block on them nor
        # leave the worker running.
        text = HEAD + 'actions = ["v"]\n[action.v]\nvia = ["--version"]\nexit = 0\nenv = { FAKE_VIA = "hang" }\n'
        start = time.monotonic()
        o = self.run_folder(self.case(text), timeout=3)
        self.assertLess(time.monotonic() - start, 30)
        self.assertFailsWith(o, "U1", "no exit within 3 s")
        pid_file = o.kept / "hang.pid"
        self.assertTrue(pid_file.is_file())
        pid = int(pid_file.read_text(encoding="utf-8"))
        deadline = time.monotonic() + 5
        while _pid_alive(pid, pid_file.stat().st_mtime) and time.monotonic() < deadline:
            time.sleep(0.05)
        self.assertFalse(_pid_alive(pid, pid_file.stat().st_mtime), "the worker outlived the timeout")

    def test_kept_on_failure(self) -> None:
        o = self.build_with(env="checkfail")
        self.assertIsNotNone(o.kept)
        self.assertTrue(o.kept.is_dir())


class LeakTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="via-conformance-leak-"))

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def scan(self, name: str, text: str) -> list[str]:
        p = self.tmp / name
        write(p, text)
        return leaks.scan_file(p, name, leaks.Secrets.for_run(self.tmp, "via-test-x"))

    def test_wire_style_working_folder(self) -> None:
        secrets = leaks.Secrets.for_run(self.tmp, "via-test-x")
        self.assertIn(self.tmp.as_posix(), secrets.texts)
        self.assertIn(self.tmp.resolve().as_posix(), secrets.texts)
        self.assertTrue(self.scan("a.txt", f"at {self.tmp.resolve().as_posix()}/w\n"))

    def test_quoted_toml_paths(self) -> None:
        for text in ('p = "/tmp/x"\n', "p = 'C:/x'\n", 'p = ["C:\\\\x"]\n', 'p = { q = "//host/x" }\n'):
            got = self.scan("a.toml", text)
            self.assertTrue(any("absolute path" in g for g in got), (text, got))
        self.assertEqual(self.scan("a.toml", 'p = "fields/a.npy"\nq = 1\n'), [])

    def test_plain_text_tokens(self) -> None:
        for text in ('path="/tmp/x"\n', "x (C:/y) z\n", "list [/a, b]\n", "k='\\\\srv\\s'\n"):
            got = self.scan("a.log", text)
            self.assertTrue(any("absolute path" in g for g in got), (text, got))
        self.assertEqual(self.scan("a.log", "fields/a.npy = 3, ok\n"), [])


class MatchingTests(unittest.TestCase):
    def test_assignment_needs_search(self) -> None:
        # Greedy first-fit gives the line-1 record to `any`, leaving `one` unmatched.
        recs = [{"code": "a.b", "line": 1, "severity": "error"}, {"code": "a.b", "line": 2, "severity": "error"}]
        doc = diags.DiagDoc(1, recs, [])
        exps = {"any": {"code": "a.b"}, "one": {"code": "a.b", "line": 1}}
        self.assertEqual(diags.match(exps, doc, "exact"), [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "records": 3}}, doc, "includes"))

    def test_count_defaults_to_one(self) -> None:
        doc = diags.DiagDoc(0, [{"code": "a.b"}], [])
        self.assertEqual(diags.match({"x": {"code": "a.b", "count": 1}}, doc, "exact"), [])

    def test_chain_entries_include(self) -> None:
        # 3.5: same length and order; each entry includes the members written, so `owner` may go unwritten.
        a = {"through": "layer", "name": "roads", "file": "layers/roads.toml", "line": 9, "owner": "roads team"}
        b = {"through": "change", "name": "fix", "file": "changes/fix.toml", "line": 2}
        doc = diags.DiagDoc(1, [{"code": "a.b", "chain": [a, b]}], [])
        want = [{k: v for k, v in a.items() if k != "owner"}, b]
        self.assertEqual(diags.match({"x": {"code": "a.b", "chain": want}}, doc, "exact"), [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "chain": [b, want[0]]}}, doc, "exact"))
        self.assertTrue(diags.match({"x": {"code": "a.b", "chain": [b]}}, doc, "exact"))


class RecordTests(Base):
    def test_record_and_compare(self) -> None:
        folder = self.case(BUILD)
        out = self.run_folder(folder)
        self.assertPass(out)
        rec_dir = self.tmp / "rec"
        a = record.write(rec_dir / "a", ("macos", "aarch64"), 1, {"build/c": out.record})
        b = record.write(rec_dir / "b", ("linux", "x86_64"), 1, {"build/c": out.record})
        self.assertTrue(canon.is_canonical(a.read_bytes()))
        self.assertEqual(record.compare(a, b)[0], [])
        changed = json.loads(b.read_text(encoding="utf-8"))
        changed["cases"]["build/c"]["actions"]["build"]["exit"] = 1
        b.write_bytes(canon.canonical_bytes(changed))
        self.assertEqual(len(record.compare(a, b)[0]), 1)

    def test_cli_run(self) -> None:
        self.case(BUILD)
        buf = io.StringIO()
        with redirect_stdout(buf):
            code = cli.main(["run", "--via", str(self.via), "--milestone", "1", "--repo", str(self.repo),
                             "--work-root", str(self.work_root), "--record", str(self.tmp / "rec")])
        self.assertEqual(code, 0, buf.getvalue())
        self.assertIn("1 passed, 0 failed, 0 pending, 0 in error", buf.getvalue())
        self.assertEqual(len(list((self.tmp / "rec").glob("*.json"))), 1)
        self.assertEqual(list(self.work_root.iterdir()), [], "a passing case's folders are deleted")


class RepositoryCases(unittest.TestCase):
    """The repository's own cases: every case.toml parses (conformance.md 7, first check), and the
    ford case folders hold exactly the files the documents give."""

    def test_every_case_parses(self) -> None:
        found = model.discover(cli.REPO / "spec" / "cases")
        self.assertTrue(found)
        errors = [f"{d.chapter}/{d.folder.name}: {d.error}" for d in found if d.case is None]
        self.assertEqual(errors, [])
        for d in found:
            self.assertTrue((cli.REPO / "spec" / f"{d.chapter}.md").is_file(), d.chapter)

    def test_ford_fixtures_current(self) -> None:
        self.assertEqual(fixtures.check(cli.REPO), [])

    def test_ford_positions_the_chapters_cite(self) -> None:
        f = fixtures.ford_files(cli.REPO)
        world = f["world.toml"]
        line64 = world.split(b"\n")[63]
        start = world.index(b"[scatter.trees]")
        self.assertEqual((line64, start, start + len(line64)), (b"[scatter.trees]", 1711, 1726))  # diagnostics.md 8
        typo = f["changes/bad_typo.toml"]
        self.assertEqual((typo.index(b"xx"), typo.index(b"xx") - typo.index(b"\n")), (82, 67))  # diagnostics.md 8
        lines = world.split(b"\n")
        self.assertEqual(lines[73], b'on = { ref = "beck", t = 0.8 }')  # build.md 11.2's line 74
        self.assertTrue(lines[24].startswith(b"on = { ref"))  # graph.md cycle two, 25:8
        self.assertEqual(lines[29], b'ends_at = "ford"')  # graph.md cycle two, line 30


if __name__ == "__main__":
    unittest.main()
