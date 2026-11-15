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

    def test_derive_and_item_in_checks(self) -> None:
        """conformance.md 3.3 `derive`, and `{item}` in a for_each action's named check arguments."""
        net = canon.canonical_bytes(_cross_network()).decode()
        inp = canon.canonical_bytes(CROSS_IN).decode()
        o = self.run_folder(self.case(HEAD + """\
            actions = ["net", "inp", "renode", "cells", "each"]
            [action.net]
            write = "net.json"
            from  = "later/net.json"
            [action.inp]
            write = "in.json"
            from  = "later/in.json"
            [action.renode]
            derive = "again.json"
            from   = "net.json"
            as     = "segments"
            [action.cells]
            derive = "cells"
            from   = "net.json"
            as     = "cells"
            input  = "in.json"
            params = { cell_length = 10000 }
            [action.each]
            via      = ["--version"]
            for_each = "cells/*.json"
            exit     = 0
            [action.each.check.n]
            fn      = "json_count"
            path    = "{item}"
            pointer = "/params/frontage"
            count   = 3
            [action.each.file.again]
            path    = "again.json"
            pointer = "/shapes/11/source"
            value   = "s11"
            """, files={"later/net.json": net, "later/in.json": inp}))
        self.assertPass(o)
        bad = self.case(HEAD + """\
            actions = ["d"]
            [action.d]
            derive = "x.json"
            from   = "net.json"
            as     = "cells"
            """)
        with self.assertRaises(model.CaseError):
            model.parse_case(bad, "build")

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

    def plant_build_leftovers(self) -> Path:
        """What a hand build leaves in an example folder: a store, a build directory, a run
        report, a lock, and a file git ignores."""
        w = self.repo / "examples" / "w"

        def write(path: Path, text: str) -> None:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(text.encode("utf-8"))

        write(w / ".via" / "store" / "index.json", '{"stale":true}\n')
        write(w / ".via" / "store" / "nodes" / "merge.bin", "stale")
        write(w / "build" / "t" / ".previous", "stale")
        write(w / "build" / "t" / "manifest.json", "{}\n")
        write(w / "build" / "t.run.json", "{}\n")
        write(w / "build.lock", "")
        write(w / "t.run.json", "{}\n")
        write(w / "w.lock", "")
        write(w / "scratch.tmp", "ignored by git")
        write(w / "sub" / "kept.toml", "x = 1\n")
        write(self.repo / ".gitignore", "*.tmp\n")
        return w

    def test_copy_skips_what_builds_leave(self) -> None:
        """A `[copy]` of a folder holding a `.via` store copies it without the store (conformance.md
        3.2): a case must start cold whatever was built in the example by hand."""
        from via_conformance.runner import _copy_into

        w = self.plant_build_leftovers()
        if shutil.which("git"):
            import subprocess
            subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        dst = self.tmp / "copied"
        _copy_into(w, dst, repo=self.repo)
        got = sorted(p.relative_to(dst).as_posix() for p in dst.rglob("*"))
        want = ["sub", "sub/kept.toml", "world.toml"]
        if not shutil.which("git"):
            want.insert(0, "scratch.tmp")
        self.assertEqual(got, want)

    def test_copied_store_is_not_seen_by_the_build(self) -> None:
        self.plant_build_leftovers()
        o = self.run_folder(self.case(BUILD + """\
            [action.build.diag.first]
            code = "identity.no_previous"
            from = "run"
            [action.build.nodes]
            ran = ["field:*", "merge"]
            [action.build.file.lock]
            path   = "w.lock"
            exists = false
            [action.build.file.report]
            path   = "t.run.json"
            exists = false
            """))
        self.assertPass(o)


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


class NamedCheckTests(unittest.TestCase):
    """The named checks added with M1's cases (conformance.md 3.4), each seen to hold and to fail."""

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="via-named-"))

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_check(self, table: dict, stdout: bytes = b"") -> list:
        from via_conformance import named
        return named.run_named("c", table, self.tmp, {"u5": lambda: [], "stdout": stdout})

    def test_vectors_match(self) -> None:
        write(self.tmp / "v.toml", '[vector.a]\nfn = "f"\nexpected = "0xAB"\n[vector.b]\nfn = "g"\nexpected = 1.5\n')
        good = b'{"got":"0xab","id":"a"}\n{"got":1.5,"id":"b"}\n'
        self.assertEqual(self.run_check({"fn": "vectors_match", "path": "v.toml"}, good), [])
        self.assertTrue(self.run_check({"fn": "vectors_match", "path": "v.toml"}, b'{"got":"0xAC","id":"a"}\n'))
        self.assertEqual(self.run_check({"fn": "vectors_match", "path": "v.toml", "fns": ["f"]}, b'{"got":"0xAB","id":"a"}\n'), [])

    def test_json_count(self) -> None:
        write(self.tmp / "o.json", '{"a":[1,2,3],"b":{"x":1}}\n')
        self.assertEqual(self.run_check({"fn": "json_count", "path": "o.json", "pointer": "/a", "count": 3}), [])
        self.assertEqual(self.run_check({"fn": "json_count", "path": "o.json", "pointer": "/b", "count": 1}), [])
        self.assertTrue(self.run_check({"fn": "json_count", "path": "o.json", "pointer": "/a", "count": 2}))
        self.assertTrue(self.run_check({"fn": "json_count", "path": "o.json", "pointer": "/c", "count": 0}))

    def test_names_count_and_kept(self) -> None:
        sq = lambda x, y: {"outer": [[x, y], [x + 1, y], [x + 1, y + 1]], "holes": []}
        a = {"features": [{"name": "g#a1", "pieces": [sq(0, 0)]}, {"name": "g#a2", "pieces": [sq(50, 50)]},
                          {"name": "r", "points": [[0, 0], [9, 9]]}]}
        b = {"features": [{"name": "g#a1", "pieces": [sq(0, 0)]}, {"name": "g#a3", "pieces": [sq(50, 50)]}]}
        write(self.tmp / "a.json", json.dumps(a))
        write(self.tmp / "b.json", json.dumps(b))
        write(self.tmp / "n.txt", "h#1\nh#2\nk#1\n")
        self.assertEqual(self.run_check({"fn": "names_count", "path": "a.json", "prefix": "g#a", "count": 2}), [])
        self.assertEqual(self.run_check({"fn": "names_count", "path": "n.txt", "prefix": "h#", "count": 2}), [])
        self.assertTrue(self.run_check({"fn": "names_count", "path": "a.json", "prefix": "g#a", "count": 3}))
        kept = {"fn": "names_kept", "first": "a.json", "second": "b.json", "prefix": "g#"}
        self.assertTrue(self.run_check(kept))
        self.assertEqual(self.run_check({**kept, "except_within": [40, 40, 60, 60]}), [])
        # Edges excluded: a feature touching the box only at its corner is still checked.
        self.assertTrue(self.run_check({**kept, "except_within": [51, 51, 60, 60]}))
        self.assertTrue(self.run_check({**kept, "prefix": "z#"}))

    def report(self, nodes: list, totals: dict) -> None:
        doc = {"build": {}, "diagnostics": [], "engine": {"epoch": 1, "version": "0"}, "format": "via-run-0", "nodes": nodes,
               "steps": [], "system": {}, "threads": 1, "totals": totals}
        (self.tmp / "r.json").write_bytes(canon.canonical_bytes(doc))

    def test_run_report_valid_and_node(self) -> None:
        nodes = [{"cause": {"reason": "statements"}, "id": "a", "status": "ran"}, {"cause": None, "id": "b", "status": "reused"}]
        self.report(nodes, {"ms": 0, "nodes": 2, "ran": 1, "recomputed": 0, "reused": 1, "skipped": 0})
        self.assertEqual(self.run_check({"fn": "run_report_valid", "path": "r.json"}), [])
        self.assertEqual(self.run_check({"fn": "run_node", "path": "r.json", "node": "a", "includes": {"cause": {"reason": "statements"}}}), [])
        self.assertTrue(self.run_check({"fn": "run_node", "path": "r.json", "node": "b", "includes": {"status": "ran"}}))
        self.report(list(reversed(nodes)), {"ms": 0, "nodes": 2, "ran": 2, "recomputed": 0, "reused": 0, "skipped": 0})
        got = self.run_check({"fn": "run_report_valid", "path": "r.json"})
        self.assertTrue(any("id order" in g[1] for g in got), got)
        self.assertTrue(any("totals.ran" in g[1] for g in got), got)

    def test_array_window(self) -> None:
        whole = np.arange(30, dtype="<f4").reshape(5, 6)
        np.save(self.tmp / "w.npy", whole)
        np.save(self.tmp / "p.npy", np.ascontiguousarray(whole[1:3, 2:5]))
        self.assertEqual(self.run_check({"fn": "array_window", "whole": "w.npy", "part": "p.npy", "window": [2, 1, 5, 3]}), [])
        self.assertTrue(self.run_check({"fn": "array_window", "whole": "w.npy", "part": "p.npy", "window": [1, 1, 4, 3]}))


    def changes(self, entries: list) -> None:
        doc = {"against": None, "entries": entries, "format": "via-changes-0", "summary": []}
        (self.tmp / "c.json").write_bytes(canon.canonical_bytes(doc))

    def test_json_list(self) -> None:
        self.changes([{"name": "cottage", "table": "instance", "via": ["shape:ford/v:build", "instance:cottage/v:build"]},
                      {"name": "trees#s7000_1", "table": "instance"}, {"name": "trees#s7000_2", "table": "instance"}])
        ok = {"fn": "json_list", "path": "c.json", "pointer": "/entries", "from": 1, "includes": {"table": "instance"},
              "starts": {"name": "trees#"}}
        self.assertEqual(self.run_check(ok), [])
        self.assertTrue(self.run_check({**ok, "from": 0}))
        self.assertTrue(self.run_check({**ok, "from": 3}))
        self.assertEqual(self.run_check({**ok, "from": 3, "at_least": 0}), [])
        via = {"fn": "json_list", "path": "c.json", "pointer": "/entries/0/via", "at_least": 2, "last": "instance:cottage/v:build"}
        self.assertEqual(self.run_check(via), [])
        self.assertTrue(self.run_check({**via, "last": "shape:ford/v:build"}))
        self.assertTrue(self.run_check({**via, "at_least": 3}))
        # `nulls`: a member present and null (graph.md 6.4's causes against another store's build).
        self.changes([{"name": "beck", "causes": None}, {"name": "ford", "causes": None}])
        nulls = {"fn": "json_list", "path": "c.json", "pointer": "/entries", "nulls": ["causes"]}
        self.assertEqual(self.run_check(nulls), [])
        self.changes([{"name": "beck", "causes": None}, {"name": "ford", "causes": []}])
        self.assertTrue(self.run_check(nulls))
        self.changes([{"name": "beck"}])
        self.assertTrue(self.run_check(nulls))

    def test_instance_at(self) -> None:
        b = self.tmp / "b" / "instances"
        b.mkdir(parents=True)
        np.save(b / "x.npy", np.array([200000, 5], dtype="<i4"))
        np.save(b / "y.npy", np.array([60000, 5], dtype="<i4"))
        write(b / "names.txt", "oak\ntrees#s7000_1\n")
        self.assertEqual(self.run_check({"fn": "instance_at", "build": "b", "name": "oak", "at": [200, 60]}), [])
        self.assertTrue(self.run_check({"fn": "instance_at", "build": "b", "name": "oak", "at": [200, 60.001]}))
        self.assertTrue(self.run_check({"fn": "instance_at", "build": "b", "name": "elm", "at": [200, 60]}))

    def test_changes_inside_and_absent(self) -> None:
        clear = [{"cause": "clear.gap", "file": "world.toml", "line": 81}]
        self.changes([{"before": {"at": [205, 206, 1]}, "causes": clear, "name": "trees#a", "what": ["removed"]},
                      {"before": {"at": [100, 100, 1]}, "causes": [{"cause": "pin.oak"}], "name": "trees#b", "what": ["removed"]}])
        inside = {"fn": "changes_inside", "path": "c.json", "prefix": "trees#", "at": [208, 208], "radius": 6,
                  "what": ["removed"], "cause": {"cause": "clear.gap", "line": 81}}
        self.assertEqual(self.run_check(inside), [])
        self.assertTrue(self.run_check({**inside, "cause": {"cause": "clear.gap", "line": 80}}))
        self.assertTrue(self.run_check({**inside, "at": [0, 0]}))
        self.assertTrue(self.run_check({**inside, "radius": 200}))  # trees#b, inside now, has another cause
        absent = {"fn": "changes_absent", "path": "c.json", "entry_names": ["oak", "beck"]}
        self.assertEqual(self.run_check(absent), [])
        self.assertTrue(self.run_check({**absent, "entry_names": ["trees#b"]}))
        self.assertTrue(self.run_check({**absent, "causes": ["pin.oak"]}))
        self.assertEqual(self.run_check({**absent, "causes": ["pin.elm"]}), [])


CROSS_IN = {"@format": "via-kernel-1", "grid": {"extent": [0, 0, 100000, 100000], "resolution": 500}, "shapes": [
    {"ring": [[0, 0], [100000, 0], [100000, 100000], [0, 100000]], "source": "frame_a"},
    {"path": [[0, 50000], [100000, 50000]], "source": "cross_ew"},
    {"path": [[50000, 0], [50000, 100000]], "source": "cross_ns"}]}


def _cross_network() -> dict:
    """geometry.md 5's cross, written out by hand from the chapter: 9 vertices, 12 segments, 5
    junctions, 8 edges, 4 areas (each a 50 m square), 1 component."""
    P = [[x, y] for x in (0, 50000, 100000) for y in (0, 50000, 100000)]
    P.sort()
    segs = []
    def prov(src, seg, t, rev=False):
        return {"provenance": [{"reversed": rev, "segment": seg, "source": src, "t": t}]}
    for a, b, pr in (
        ([0, 0], [0, 50000], prov("frame_a", 3, [1, 0.875], True)), ([0, 0], [50000, 0], prov("frame_a", 0, [0, 0.125])),
        ([0, 50000], [0, 100000], prov("frame_a", 3, [0.875, 0.75], True)), ([0, 50000], [50000, 50000], prov("cross_ew", 0, [0, 0.5])),
        ([0, 100000], [50000, 100000], prov("frame_a", 2, [0.75, 0.625], True)), ([50000, 0], [50000, 50000], prov("cross_ns", 0, [0, 0.5])),
        ([50000, 0], [100000, 0], prov("frame_a", 0, [0.125, 0.25])), ([50000, 50000], [50000, 100000], prov("cross_ns", 0, [0.5, 1])),
        ([50000, 50000], [100000, 50000], prov("cross_ew", 0, [0.5, 1])), ([50000, 100000], [100000, 100000], prov("frame_a", 2, [0.625, 0.5], True)),
        ([100000, 0], [100000, 50000], prov("frame_a", 1, [0.25, 0.375])), ([100000, 50000], [100000, 100000], prov("frame_a", 1, [0.375, 0.5])),
    ):
        segs.append({"ends": [a, b], **pr})
    sq = lambda x, y: [[[x, y], [x + 50000, y], [x + 50000, y + 50000], [x, y + 50000]]]
    areas = [{"region": [sq(x, y)], "twice_area": 5000000000} for x, y in ((0, 0), (0, 50000), (50000, 0), (50000, 50000))]
    junctions = [[0, 50000], [50000, 0], [50000, 50000], [50000, 100000], [100000, 50000]]
    paths = [
        [[0, 50000], [50000, 50000]], [[50000, 50000], [100000, 50000]], [[50000, 0], [50000, 50000]],
        [[50000, 50000], [50000, 100000]], [[0, 50000], [0, 0], [50000, 0]], [[50000, 0], [100000, 0], [100000, 50000]],
        [[0, 50000], [0, 100000], [50000, 100000]], [[50000, 100000], [100000, 100000], [100000, 50000]],
    ]
    edges = sorted(({"ends": [p[0], p[-1]], "path": p} for p in paths), key=lambda e: (e["ends"], e["path"][1]))
    return {"vertices": P, "segments": segs, "junctions": junctions, "edges": edges, "areas": areas, "components": 1}


class KernelCheckTests(unittest.TestCase):
    """The kernel guarantees of geometry.md 3, each seen to hold and to fail."""

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="via-kernel-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.put("in.json", CROSS_IN)

    def put(self, name: str, doc: dict) -> None:
        (self.tmp / name).write_bytes(canon.canonical_bytes(doc))

    def run_check(self, table: dict) -> list:
        from via_conformance import named
        self.notes: list[str] = []
        return named.run_named("c", table, self.tmp, {"u5": lambda: [], "stdout": b"", "note": self.notes.append})

    def test_network_valid(self) -> None:
        net = _cross_network()
        self.put("out.json", net)
        check = {"fn": "network_valid", "path": "out.json", "input": "in.json"}
        self.assertEqual(self.run_check(check), [])
        bad = json.loads(json.dumps(net))
        bad["areas"] = bad["areas"][1:]  # a face with no area: Euler and Closure both fail
        self.put("out.json", bad)
        got = self.run_check(check)
        self.assertTrue(any("Euler, fine" in g[1] for g in got), got)
        self.assertTrue(any("Closure" in g[1] for g in got), got)
        bad = json.loads(json.dumps(net))
        bad["segments"][0]["ends"] = [[0, 0], [100000, 100000]]  # a diagonal through a vertex
        self.put("out.json", bad)
        self.assertTrue(any("3.1 Noded" in g[1] for g in self.run_check(check)))
        bad = json.loads(json.dumps(net))
        bad["junctions"] = bad["junctions"][:4]
        self.put("out.json", bad)
        self.assertTrue(any("junctions" in g[1] for g in self.run_check(check)))
        bad = json.loads(json.dumps(net))
        bad["areas"][0]["region"][0][0].reverse()  # a clockwise shell
        self.put("out.json", bad)
        self.assertTrue(self.run_check(check))

    def test_course_valid_within(self) -> None:
        feat = {"name": "course", "points": [[0, 7], [1, 7.9], [7, 8], [8, 7]]}
        self.put("f.json", {"features": [feat]})
        check = {"fn": "course_valid", "path": "f.json", "feature": "course", "start": [0, 7],
                 "end": [8, 7], "min_segment": 0.001, "within": [0, 0, 8, 8]}
        self.assertEqual(self.run_check(check), [])  # y = 8 is on the edge: inside
        feat["points"][2] = [7, 8.01]
        self.put("f.json", {"features": [feat]})
        got = self.run_check(check)
        self.assertTrue(any("vertex 2 within" in g[1] for g in got), got)

    def test_network_measures_never_fails(self) -> None:
        self.put("out.json", _cross_network())
        self.put("ref.json", {"streets": {"faces": 5, "total_area_m2": 1}})
        self.assertEqual(self.run_check({"fn": "network_measures", "path": "out.json", "reference": "ref.json"}), [])
        self.assertIn("4 areas against 5 faces", self.notes[0])

    def test_noding_valid_and_same(self) -> None:
        inp = {"@format": "via-kernel-1", "shapes": [{"path": [[0, 0], [10, 0]], "source": "a"},
                                                     {"path": [[5, -5], [5, 5]], "source": "b"}]}
        self.put("n.json", inp)
        out = {"vertices": [[0, 0], [5, -5], [5, 0], [5, 5], [10, 0]], "segments": [
            {"ends": [[0, 0], [5, 0]], "provenance": [{"reversed": False, "segment": 0, "source": "a", "t": [0, 0.5]}]},
            {"ends": [[5, -5], [5, 0]], "provenance": [{"reversed": False, "segment": 0, "source": "b", "t": [0, 0.5]}]},
            {"ends": [[5, 0], [5, 5]], "provenance": [{"reversed": False, "segment": 0, "source": "b", "t": [0.5, 1]}]},
            {"ends": [[5, 0], [10, 0]], "provenance": [{"reversed": False, "segment": 0, "source": "a", "t": [0.5, 1]}]}]}
        self.put("o.json", out)
        check = {"fn": "noding_valid", "path": "o.json", "input": "n.json"}
        self.assertEqual(self.run_check(check), [])
        far = json.loads(json.dumps(out))
        far["segments"][0]["ends"] = [[0, 0], [5, 2]]
        far["segments"][3]["ends"] = [[5, 2], [10, 0]]
        self.put("o.json", far)
        self.assertTrue(any("Close" in g[1] for g in self.run_check(check)))
        gap = json.loads(json.dumps(out))
        del gap["segments"][3]
        self.put("o.json", gap)
        self.assertTrue(any("Faithful" in g[1] for g in self.run_check(check)))
        uncut = {"vertices": out["vertices"], "segments": [
            {"ends": [[0, 0], [10, 0]], "provenance": [{"reversed": False, "segment": 0, "source": "a", "t": [0, 1]}]},
            {"ends": [[5, -5], [5, 5]], "provenance": [{"reversed": False, "segment": 0, "source": "b", "t": [0, 1]}]}]}
        self.put("o.json", uncut)
        self.assertTrue(any("Noded" in g[1] for g in self.run_check(check)))
        self.put("o.json", out)
        self.put("p.json", out)
        same = {"fn": "noding_same", "first": "o.json", "second": "p.json"}
        self.assertEqual(self.run_check(same), [])
        self.put("p.json", gap)
        self.assertTrue(self.run_check(same))

    def test_names_same(self) -> None:
        a = {"junctions": [{"name": "j1"}], "edges": [{"name": "e1"}], "areas": [{"name": "a1"}, {"name": "a2"}]}
        self.put("a.json", a)
        self.put("b.json", {**a, "areas": [{"name": "a2"}, {"name": "a1"}]})
        self.assertEqual(self.run_check({"fn": "names_same", "first": "a.json", "second": "b.json"}), [])
        self.put("b.json", {**a, "areas": [{"name": "a3"}, {"name": "a1"}]})
        self.assertTrue(self.run_check({"fn": "names_same", "first": "a.json", "second": "b.json"}))

    def test_derive_and_cells_valid(self) -> None:
        from via_conformance import kernel
        net = _cross_network()
        made = kernel.derive_cells(net, CROSS_IN, {"cell_length": 10000})
        self.assertEqual(len(made), 4)
        first = made[0][1]["params"]
        self.assertEqual(first["area"], [0, 0])
        # The first area, [0, 0]-[50, 50]: inside frame_a, a ring's left (geometry.md 3.2); below
        # cross_ew, which runs east (its right); west of cross_ns, which runs north (its left).
        sides = {(f["source"], f["side"]) for f in first["frontage"]}
        self.assertEqual(sides, {("frame_a", "left"), ("cross_ew", "right"), ("cross_ns", "left")})
        seg = kernel.derive_segments(net)
        self.assertEqual(len(seg["shapes"]), 12)
        self.assertEqual(seg["shapes"][0], {"path": [[0, 0], [0, 50000]], "source": "s00"})
        # cells_valid: one 50 m cell along cross_ns, 10 m deep, and the rest.
        net["edges"] = [{"ends": [[50000, 0], [50000, 50000]], "path": [[50000, 0], [50000, 50000]]}]
        self.put("net.json", net)
        cin = {"params": {"area": [0, 0], "min_cell_length": 8000}}
        self.put("cin.json", cin)
        cell = [[[40000, 0], [50000, 0], [50000, 50000], [40000, 50000]]]
        rest = [[[0, 0], [40000, 0], [40000, 50000], [0, 50000]]]
        fr = {"edge": [[50000, 0], [50000, 50000]], "side": "left", "source": "cross_ns"}
        self.put("cout.json", {"cells": [{"frontage": fr, "region": [cell], "station": 0}], "rest": [rest], "diagnostics": []})
        check = {"fn": "cells_valid", "path": "cout.json", "input": "cin.json", "network": "net.json"}
        self.assertEqual(self.run_check(check), [])
        short = [[[40000, 0], [50000, 0], [50000, 5000], [40000, 5000]]]
        self.put("cout.json", {"cells": [{"frontage": fr, "region": [short], "station": 0}], "rest": [rest], "diagnostics": []})
        got = self.run_check(check)
        self.assertTrue(any("Fronts" in g[1] for g in got), got)
        self.assertTrue(any("Closure" in g[1] for g in got), got)
        self.put("cout.json", {"cells": [], "rest": [], "diagnostics": []})
        self.assertTrue(any("cell_none" in g[1] for g in self.run_check(check)))

    def test_sample_terrain(self) -> None:
        b = self.tmp / "b" / "fields"
        b.mkdir(parents=True)
        arr = np.array([[0, 1, 2], [3, 4, 6]], dtype="<f4")  # rows from the south, 3 by 2 posts
        np.save(b / "height.npy", arr)
        self.put("b/manifest.json", {"frame": {"extent": [0, 0, 2, 1], "posts": [3, 2], "resolution": 1}})
        self.put("q.json", {"surface": "height", "x": [500, 2000], "y": [0, 1000]})
        # (0.5, 0): cell (0, 0), z = 0.5, gx = 1, gy = 3. (2, 1): the last cell (1, 0), z = 6,
        # gx = (1 * 0 + 2 * 1) / 1 = 2, gy = (3 * 0 + 4 * 1) / 1 = 4.
        self.put("a.json", {"gx": [1, 2], "gy": [3, 4], "inside": [True, True], "z": [0.5, 6]})
        check = {"fn": "sample_terrain", "path": "a.json", "input": "q.json", "field": "b/fields/height.npy",
                 "manifest": "b/manifest.json"}
        self.assertEqual(self.run_check(check), [])
        self.put("a.json", {"gx": [1, 1], "gy": [3, 4], "inside": [True, True], "z": [0.5, 6]})
        self.assertTrue(self.run_check(check))


class HarnessTests(unittest.TestCase):
    """BLAKE3, the id rule, the round trips, the stroke measure and the store flip (conformance.md
    3.3, 3.4, 3.7), each seen to hold and to fail."""

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="via-harness-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)

    def put(self, name: str, doc: dict) -> None:
        (self.tmp / name).parent.mkdir(parents=True, exist_ok=True)
        (self.tmp / name).write_bytes(canon.canonical_bytes(doc))

    def run_check(self, table: dict) -> list:
        from via_conformance import named
        self.notes: list[str] = []
        return named.run_named("c", table, self.tmp, {"u5": lambda: [], "stdout": b"", "note": self.notes.append})

    def test_blake3_vectors(self) -> None:
        from via_conformance.blake3 import digest
        # The reference implementation's digests of the empty input and "abc". Inputs of several
        # chunks (a tree of parents) are checked by test_id_rule_on_the_null_vector's engine-made
        # id only up to one chunk; larger ones were checked against store object names by hand.
        self.assertEqual(digest(b""), "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262")
        self.assertEqual(digest(b"abc"), "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85")

    def test_id_rule_on_the_null_vector(self) -> None:
        null = HERE.parents[2] / "spec/cases/build/c01_null/expected"
        (self.tmp / "b").mkdir()
        shutil.copy(null / "manifest.json", self.tmp / "b/manifest.json")
        self.assertEqual(self.run_check({"fn": "id_rule", "build": "b"}), [])
        m = canon.loads((self.tmp / "b/manifest.json").read_bytes())
        self.assertEqual(m["id"], (null / "id.txt").read_text().strip())
        m["seed"] = "0000000000000001" if m.get("seed") != "0000000000000001" else "0000000000000002"
        self.put("b/manifest.json", m)
        self.assertTrue(self.run_check({"fn": "id_rule", "build": "b"}))

    def test_roundtrip_point(self) -> None:
        line = {"@format": "via-kernel-1", "points": [[5000, 3000], [12000, -50]],
                "shapes": [{"path": [[0, 0], [10000, 0], [10000, 7000]], "source": "p"}]}
        self.put("in.json", line)
        self.put("out.json", {"results": [{"bearing": 0, "point": [5000, 1]}, {"bearing": 0, "point": [10000, 0]}]})
        check = {"fn": "roundtrip_point", "path": "out.json", "input": "in.json", "bound": 1}
        self.assertEqual(self.run_check(check), [])
        self.put("out.json", {"results": [{"bearing": 0, "point": [5000, 2]}, {"bearing": 0, "point": [10000, 0]}]})
        self.assertTrue(self.run_check(check))
        self.put("out.json", {"results": [{"bearing": 0, "point": [5000, 0]}]})
        self.assertTrue(self.run_check(check))

    def test_roundtrip_t(self) -> None:
        self.put("in.json", {"@format": "via-kernel-1", "t": [0.5],
                             "shapes": [{"path": [[0, 0], [10000, 0]], "source": "p"}]})
        check = {"fn": "roundtrip_t", "path": "out.json", "input": "in.json"}
        self.put("out.json", {"results": [{"d": 0, "nearest": [5000, 0], "sd": 0, "t": 0.50009}]})
        self.assertEqual(self.run_check(check), [])
        self.put("out.json", {"results": [{"d": 0, "nearest": [5000, 0], "sd": 0, "t": 0.5002}]})
        self.assertTrue(self.run_check(check))

    def test_stroke_measures_never_fails(self) -> None:
        sq = [[[[0, 0], [10000, 0], [10000, 10000], [0, 10000]], [[2000, 2000], [2000, 4000], [4000, 4000], [4000, 2000]]]]
        self.put("out.json", {"results": [sq]})
        self.put("ref.json", {"river_offsets": {"offsets": [{"distance_m": 8.5, "area_m2": 99}]}})
        self.assertEqual(self.run_check({"fn": "stroke_measures", "path": "out.json", "reference": "ref.json",
                                         "distance": 8.5}), [])
        self.assertIn("96.00 m2 against 99 m2", self.notes[0])

    def test_derive_feeds_one_call_to_the_next(self) -> None:
        from via_conformance import kernel
        inp = {"@format": "via-kernel-1", "points": [[1, 2]], "shapes": [{"path": [[0, 0], [9, 0]], "source": "p"}]}
        doc = kernel.derive_interpolate({"results": [{"t": 0.25}]}, inp)
        self.assertEqual(doc, {"@format": "via-kernel-1", "shapes": inp["shapes"], "t": [0.25]})
        doc = kernel.derive_project({"results": [{"bearing": 0, "point": [3, 0]}]}, inp)
        self.assertEqual(doc["points"], [[3, 0]])
        self.assertEqual(kernel.derive_params(inp, {"left": 1})["params"], {"left": 1})


class MatchingTests(unittest.TestCase):
    def test_assignment_needs_search(self) -> None:
        # Greedy first-fit gives the line-1 record to `any`, leaving `one` unmatched.
        recs = [{"code": "a.b", "line": 1, "severity": "error"}, {"code": "a.b", "line": 2, "severity": "error"}]
        doc = diags.DiagDoc(1, recs, [])
        exps = {"any": {"code": "a.b"}, "one": {"code": "a.b", "line": 1}}
        self.assertEqual(diags.match(exps, doc, "exact"), [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "records": 3}}, doc, "includes"))

    def test_records_zero_is_absent(self) -> None:
        doc = diags.DiagDoc(0, [{"code": "a.b"}], [{"code": "build.baseline_missing"}])
        none = {"x": {"code": "build.baseline_missing", "from": "run", "records": 0}}
        self.assertTrue(diags.match(none, doc, "includes"))
        self.assertEqual(diags.match(none, diags.DiagDoc(0, [{"code": "a.b"}], []), "includes"), [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "records": 0}}, doc, "any") == [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "records": 0}}, doc, "includes"))

    def test_count_defaults_to_one(self) -> None:
        doc = diags.DiagDoc(0, [{"code": "a.b"}], [])
        self.assertEqual(diags.match({"x": {"code": "a.b", "count": 1}}, doc, "exact"), [])

    def test_data_has_and_below(self) -> None:
        doc = diags.DiagDoc(1, [{"code": "a.b", "data": {"level": 12, "would_hold": 2.5, "gone": None}}], [])
        self.assertEqual(diags.match({"x": {"code": "a.b", "data_has": ["would_hold"]}}, doc, "includes"), [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "data_has": ["gone"]}}, doc, "includes"))
        self.assertTrue(diags.match({"x": {"code": "a.b", "data_has": ["missing"]}}, doc, "includes"))
        self.assertEqual(diags.match({"x": {"code": "a.b", "data_below": {"would_hold": "level"}}}, doc, "includes"), [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "data_below": {"level": "would_hold"}}}, doc, "includes"))
        self.assertTrue(diags.match({"x": {"code": "a.b", "data_below": {"would_hold": "missing"}}}, doc, "includes"))

    def test_position_within(self) -> None:
        doc = diags.DiagDoc(1, [{"code": "a.b", "position": [143, 178.4]}, {"code": "a.c"}], [])
        self.assertEqual(diags.match({"x": {"code": "a.b", "position_within": [135, 170, 151, 187]}}, doc, "includes"), [])
        self.assertEqual(diags.match({"x": {"code": "a.b", "position_within": [143, 178.4, 143, 178.4]}}, doc, "includes"), [])
        self.assertTrue(diags.match({"x": {"code": "a.b", "position_within": [144, 170, 151, 187]}}, doc, "includes"))
        self.assertTrue(diags.match({"x": {"code": "a.c", "position_within": [0, 0, 999, 999]}}, doc, "includes"))

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


class DefaultViaTests(unittest.TestCase):
    """The default binary follows CARGO_TARGET_DIR and is refused when older than crates/: a stale
    default once failed a correct case (documents/change_file_layer)."""

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="via-conf-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)

    def test_target_dir(self) -> None:
        import os
        old = os.environ.get("CARGO_TARGET_DIR")
        try:
            os.environ["CARGO_TARGET_DIR"] = str(self.tmp / "t")
            self.assertEqual(cli._default_via(self.tmp).parent, self.tmp / "t" / "debug")
            del os.environ["CARGO_TARGET_DIR"]
            self.assertEqual(cli._default_via(self.tmp).parent, self.tmp / "target" / "debug")
        finally:
            if old is not None:
                os.environ["CARGO_TARGET_DIR"] = old

    def test_stale(self) -> None:
        import os
        src = self.tmp / "crates" / "a" / "src" / "lib.rs"
        src.parent.mkdir(parents=True)
        write(src, "")
        via = self.tmp / "via"
        write(via, "")
        os.utime(src, (1000, 1000))
        os.utime(via, (2000, 2000))
        self.assertIsNone(cli._stale_against(via, self.tmp))
        os.utime(src, (3000, 3000))
        self.assertEqual(cli._stale_against(via, self.tmp), src)


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
