"""Running one case (conformance.md 3 to 5): the working folder, the actions, and every check.

The runner drives the `via` binary through its command line only (R20). It never links the engine
and never uses the SDK.
"""

from __future__ import annotations

import glob
import hashlib
import os
import platform
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from . import blake3, canon, diags, files, leaks, named
from . import kernel as kernel_files
from .model import Action, Case, CaseError

Failure = tuple[str, str, str]

BUILD_COMMANDS = ("build", "lock")
# build.md 11.1: when one of these is raised no build directory can be written.
NO_DIRECTORY_CODES = {"graph.busy", "build.not_a_build", "build.store_inside", "build.write_failed"}
CASE_OWN = {"case.toml", "expected", "later"}


@dataclass
class Settings:
    # The binary. A `.py` file is run by this Python: the self-tests' fake binary, which a `#!`
    # line cannot make executable on Windows.
    via: Path
    repo: Path
    milestone: int
    work_root: Path
    timeout: float = 900.0

    def argv(self) -> list[str]:
        if self.via.suffix == ".py":
            return [sys.executable, str(self.via)]
        return [str(self.via)]


@dataclass
class Outcome:
    case: str
    status: str  # pass, fail, pending, error
    detail: list[str] = field(default_factory=list)
    kept: Path | None = None
    milestone: int = 0
    record: dict[str, Any] | None = None


class _Stop(Exception):
    """An action failed: the case stops after reporting every failed check of that action."""


def system_name() -> tuple[str, str]:
    """`std::env::consts::OS` and `ARCH` as Rust names them (graph.md 6.3)."""
    osname = {"darwin": "macos", "win32": "windows"}.get(sys.platform, sys.platform)
    if osname.startswith("linux"):
        osname = "linux"
    machine = platform.machine().lower()
    arch = {"arm64": "aarch64", "amd64": "x86_64", "x64": "x86_64"}.get(machine, machine)
    return osname, arch


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _snapshot(root: Path, exclude: set[Path]) -> dict[str, tuple]:
    """Every path under root: (kind, size, mtime_ns, digest) for files, ("dir",) for folders."""
    out: dict[str, tuple] = {}
    if not root.exists():
        return out
    if root.is_file():
        st = root.stat()
        return {".": ("file", st.st_size, st.st_mtime_ns, _sha(root.read_bytes()))}
    for dirpath, dirnames, filenames in os.walk(root):
        d = Path(dirpath)
        dirnames.sort()
        for name in dirnames:
            p = d / name
            if p not in exclude:
                out[p.relative_to(root).as_posix()] = ("dir",)
        for name in sorted(filenames):
            p = d / name
            if p in exclude:
                continue
            try:
                st = p.lstat()
                out[p.relative_to(root).as_posix()] = ("file", st.st_size, st.st_mtime_ns, _sha(p.read_bytes()))
            except OSError:
                out[p.relative_to(root).as_posix()] = ("gone",)
    return out


def _diff_snapshots(before: dict[str, tuple], after: dict[str, tuple]) -> list[str]:
    changes: list[str] = []
    for k in sorted(set(before) | set(after), key=str.encode):
        if k not in after:
            changes.append(f"deleted {k}")
        elif k not in before:
            changes.append(f"created {k}")
        elif before[k] != after[k]:
            changes.append(f"changed {k}")
    return changes


# What a build leaves beside its inputs and a case never copies (conformance.md 3.2): the store
# (graph.md 3.2), the default build directory (build.md 14), the run report (graph.md 6.3) and the
# lock (graph.md 6.3). A local build in `examples/ford` leaves all four.
LEFT_BY_BUILDS_DIRS = {".via", "build"}
LEFT_BY_BUILDS_SUFFIXES = (".run.json", ".lock")


def _left_by_builds(name: str, is_dir: bool) -> bool:
    if is_dir:
        return name in LEFT_BY_BUILDS_DIRS
    return name.endswith(LEFT_BY_BUILDS_SUFFIXES)


def _git_ignored(repo: Path, paths: list[Path]) -> set[Path]:
    """The paths git ignores and does not track. Outside a git checkout, or without git, none."""
    if not paths:
        return set()
    try:
        rels = [p.resolve().relative_to(repo.resolve()).as_posix() for p in paths]
    except ValueError:
        return set()
    try:
        r = subprocess.run(
            ["git", "-C", str(repo), "check-ignore", "-z", "--stdin"],
            input="\0".join(rels).encode("utf-8") + b"\0",
            capture_output=True,
            timeout=60,
        )
    except (OSError, subprocess.SubprocessError):
        return set()
    if r.returncode not in (0, 1):  # 128: not a checkout
        return set()
    hit = {s for s in r.stdout.decode("utf-8").split("\0") if s}
    return {p for p, rel in zip(paths, rels) if rel in hit}


def _copy_into(src: Path, dst: Path, repo: Path | None = None, skip_build_output: bool = True) -> None:
    """Copy a fixture: never what a build left in it, nor anything git ignores (conformance.md 3.2).

    `skip_build_output` drops the store, build directories, run reports and locks by name; it holds
    for `[copy]` sources, whose folders are worlds someone may have built in by hand. A case's own
    files keep a planted `build/` (a case may need one) but still lose `.via` and git-ignored files.
    """
    if not src.is_dir():
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst, follow_symlinks=False)
        return
    keep_dirs: list[Path] = []
    keep_files: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(src):
        d = Path(dirpath)
        dirnames[:] = sorted(
            n for n in dirnames
            if not (n == ".via" or (skip_build_output and _left_by_builds(n, True)))
        )
        keep_dirs.extend(d / n for n in dirnames)
        keep_files.extend(
            d / n for n in sorted(filenames) if not (skip_build_output and _left_by_builds(n, False))
        )
    ignored = _git_ignored(repo, keep_dirs + keep_files) if repo is not None else set()
    dst.mkdir(parents=True, exist_ok=True)
    for p in keep_dirs:
        if p in ignored or any(a in ignored for a in p.parents):
            continue
        (dst / p.relative_to(src)).mkdir(parents=True, exist_ok=True)
    for p in keep_files:
        if p in ignored or any(a in ignored for a in p.parents):
            continue
        out = dst / p.relative_to(src)
        out.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, out, follow_symlinks=False)


class CaseRun:
    """The state of one case while it runs."""

    def __init__(self, case: Case, settings: Settings, registry: diags.Registry):
        self.case = case
        self.s = settings
        self.registry = registry
        self.token = secrets.token_hex(6)
        self.user = f"via-test-{self.token}"
        self.work = Path(
            tempfile.mkdtemp(prefix=f"via-{case.chapter}-{case.id}-{self.token}-", dir=settings.work_root)
        )
        self.siblings: list[Path] = []
        self.failures: list[tuple[str, str, str, str]] = []  # (action, check, expected, got)
        self.notes: list[str] = []
        self.results: dict[str, dict[str, Any]] = {}  # action id -> what `same` may compare
        self.record: dict[str, Any] = {}
        self.outs = [
            a.table["out"] for a in case.actions if a.kind == "via" and "out" in a.table
        ]
        # Paths that later `same` checks compare, keyed by the action they compare against.
        self.same_paths: dict[str, set[str]] = {}
        for a in case.actions:
            if a.kind != "via":
                continue
            for s in a.table.get("same", {}).values():
                if s["what"] not in ("id", "files", "diagnostics", "stdout", "ran"):
                    self.same_paths.setdefault(s["action"], set()).add(s["what"])

    # --- the working folder -----------------------------------------------------------------

    def env(self, extra: dict[str, str]) -> dict[str, str]:
        home = self.work / ".home"
        env = {
            "PATH": os.environ.get("PATH", ""),
            "VIA_LIB": str(self.s.repo / "lib"),
            "VIA_CONFIG_DIR": str(self.work / ".config"),
            "HOME": str(home),
            "USERPROFILE": str(home),
            "XDG_CONFIG_HOME": str(home / ".config"),
            "APPDATA": str(home / "appdata"),
            "TMPDIR": str(home / "tmp"),
            "TMP": str(home / "tmp"),
            "TEMP": str(home / "tmp"),
            "USER": self.user,
            "USERNAME": self.user,
            "NO_COLOR": "1",
        }
        if sys.platform == "win32":
            # Machine settings, not the user's: Windows itself and many Python packages a step's
            # worker imports fail without them (conformance.md 5.2).
            for k in ("SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT"):
                if k in os.environ:
                    env[k] = os.environ[k]
            env["LOCALAPPDATA"] = str(home / "localappdata")
        env.update(extra)
        return env

    def prepare(self) -> None:
        for c in self.case.copies:
            src = self.s.repo / c.src
            if not src.exists():
                raise CaseError(f"copy.{c.id}.from: {c.src} does not exist in the repository")
            dst = self.work if c.to == "." else self.work / c.to
            if c.to == "." and not src.is_dir():
                raise CaseError(f"copy.{c.id}: `to = \".\"` copies a folder's contents; {c.src} is a file")
            _copy_into(src, dst, repo=self.s.repo)
        for p in sorted(self.case.folder.iterdir(), key=lambda q: q.name.encode()):
            if p.name in CASE_OWN or p.name == ".via":
                continue
            _copy_into(p, self.work / p.name, repo=self.s.repo, skip_build_output=False)
        for d in (".config", ".diag", ".home", ".home/.config", ".home/appdata", ".home/localappdata", ".home/tmp"):
            (self.work / d).mkdir(parents=True, exist_ok=True)

    def subst(self, text: str, item: str | None) -> str:
        out = (
            text.replace("{work}", str(self.work))
            .replace("{case}", str(self.case.folder.resolve()))
            .replace("{repo}", str(self.s.repo))
            .replace("{python}", sys.executable)
        )
        if item is not None:
            out = out.replace("{item}", item)
        return out

    # --- edits ------------------------------------------------------------------------------

    def edit(self, a: Action) -> None:
        t = a.table
        path = self.work / a.target
        where = f"action.{a.id}"
        if a.kind == "replace":
            if not path.is_file():
                raise CaseError(f"{where}: {a.target} is not a file")
            data = path.read_bytes()
            find = t["find"].encode("utf-8")
            n = data.count(find)
            if n != 1:
                raise CaseError(f"{where}: `find` occurs {n} times in {a.target}, not once")
            path.write_bytes(data.replace(find, t["with"].encode("utf-8")))
        elif a.kind == "write":
            path.parent.mkdir(parents=True, exist_ok=True)
            if "text" in t:
                path.write_bytes(t["text"].encode("utf-8"))
            else:
                src = self.case.folder / t["from"]
                if not src.is_file():
                    raise CaseError(f"{where}.from: {t['from']} does not exist in the case folder")
                path.write_bytes(src.read_bytes())
        elif a.kind == "delete":
            if path.is_dir() and not path.is_symlink():
                shutil.rmtree(path)
            elif path.exists() or path.is_symlink():
                path.unlink()
            else:
                raise CaseError(f"{where}: {a.target} does not exist")
        elif a.kind == "flip":
            if not path.is_file():
                raise CaseError(f"{where}: {a.target} is not a file")
            data = bytearray(path.read_bytes())
            if t["at"] >= len(data):
                raise CaseError(f"{where}: byte {t['at']} is past the end of {a.target} ({len(data)} bytes)")
            data[t["at"]] ^= 0xFF
            path.write_bytes(bytes(data))
        elif a.kind == "flip_object":
            self.flip_object(a)
        elif a.kind == "rerecord":
            self.rerecord(a)
        elif a.kind == "derive":
            self.derive(a)
        elif a.kind == "truncate":
            if not path.is_file():
                raise CaseError(f"{where}: {a.target} is not a file")
            size = path.stat().st_size
            if t["bytes"] > size:
                raise CaseError(f"{where}: cannot cut {a.target} ({size} bytes) to {t['bytes']} bytes")
            with open(path, "r+b") as fh:
                fh.truncate(t["bytes"])

    def flip_object(self, a: Action) -> None:
        """conformance.md 3.3 `flip_object`: inverts byte `at` of the store object holding the value
        the run report names for `node` (graph.md 3.2: `objects/ab/<value digest>`)."""
        t = a.table
        where = f"action.{a.id}"
        report = self.work / a.target
        try:
            nodes = canon.loads(report.read_bytes())["nodes"]
        except (OSError, ValueError, KeyError, TypeError) as e:
            raise CaseError(f"{where}: {a.target} is not a run report: {e}") from e
        found = [n for n in nodes if isinstance(n, dict) and n.get("id") == t["node"]]
        if len(found) != 1 or not isinstance(found[0].get("value"), str):
            raise CaseError(f"{where}: {a.target} names no value for node {t['node']!r}")
        value = found[0]["value"]
        path = self.work / t["store"] / "objects" / value[:2] / value
        if not path.is_file():
            raise CaseError(f"{where}: the store holds no object {value} for node {t['node']!r}")
        data = bytearray(path.read_bytes())
        if t["at"] >= len(data):
            raise CaseError(f"{where}: byte {t['at']} is past the end of object {value} ({len(data)} bytes)")
        data[t["at"]] ^= 0xFF
        path.chmod(path.stat().st_mode | 0o200)
        path.write_bytes(bytes(data))

    def rerecord(self, a: Action) -> None:
        """conformance.md 3.3 `rerecord`: rewrites the manifest's record of a file of its build
        directory (`run.reports`, else `files`) to the file's bytes and BLAKE3 digest as they stand;
        nothing else of the manifest changes, and it stays canonical."""
        t = a.table
        where = f"action.{a.id}"
        mpath = self.work / t["manifest"]
        fpath = self.work / a.target
        try:
            m = canon.loads(mpath.read_bytes())
            rel = fpath.relative_to(mpath.parent).as_posix()
        except (OSError, ValueError) as e:
            raise CaseError(f"{where}: {e}") from e
        reports = m.get("run", {}).get("reports", {}) if isinstance(m, dict) else {}
        table = reports if rel in reports else m.get("files", {}) if isinstance(m, dict) else {}
        if rel not in table or not fpath.is_file():
            raise CaseError(f"{where}: {t['manifest']} records no file {rel}")
        data = fpath.read_bytes()
        table[rel] = dict(table[rel], bytes=len(data), digest=blake3.digest(data))
        mpath.write_bytes(canon.canonical_bytes(m))

    def derive(self, a: Action) -> None:
        """conformance.md 3.3 `derive`: a kernel input written from a kernel output."""
        t = a.table
        where = f"action.{a.id}"
        try:
            out = kernel_files.read(self.work / t["from"])
            inp = kernel_files.read(self.work / t["input"]) if "input" in t else None
        except kernel_files.KernelFileError as e:
            raise CaseError(f"{where}: {e}") from e
        dest = self.work / t["derive"]
        one = {
            "segments": lambda: kernel_files.derive_segments(out),
            "interpolate": lambda: kernel_files.derive_interpolate(out, inp),
            "project": lambda: kernel_files.derive_project(out, inp),
            "params": lambda: kernel_files.derive_params(out, t["params"]),
        }
        if t["as"] in one:
            try:
                doc = one[t["as"]]()
            except (kernel_files.KernelFileError, KeyError, TypeError, IndexError, ValueError) as e:
                raise CaseError(f"{where}: {t['from']} cannot be derived as {t['as']}: {e}") from e
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(canon.canonical_bytes(doc))
        else:
            if dest.exists():
                raise CaseError(f"{where}: {t['derive']} already exists")
            dest.mkdir(parents=True)
            made = kernel_files.derive_cells(out, inp, t["params"])
            if not made:
                raise CaseError(f"{where}: {t['from']} has no area")
            for fname, doc in made:
                (dest / fname).write_bytes(canon.canonical_bytes(doc))

    # --- via actions ------------------------------------------------------------------------

    def fail(self, action: str, check: str, expected: str, got: str) -> None:
        self.failures.append((action, check, expected, got))

    def run_process(self, args: list[str], cwd: Path, env: dict[str, str], holding: Path | None = None):
        lock = _hold(holding) if holding is not None else None
        try:
            return _run_bounded(self.s.argv() + args, cwd, env, self.s.timeout)
        finally:
            if lock is not None:
                _release(lock)

    def via_action(self, a: Action) -> None:
        t = a.table
        if "for_each" in t:
            # Joined as text: a Path would drop a trailing `/`, which makes a glob match folders only.
            pattern = os.path.join(str(self.work), t["for_each"])
            matches = sorted(
                (Path(m).relative_to(self.work).as_posix() for m in glob.glob(pattern, recursive=True)),
                key=str.encode,
            )
            if not matches:
                raise CaseError(f"action.{a.id}.for_each: {t['for_each']!r} matches nothing")
            for n, item in enumerate(matches, 1):
                self.via_once(a, item, f"{a.id}.{n}")
        else:
            self.via_once(a, None, a.id)

    def via_once(self, a: Action, item: str | None, run_id: str) -> None:
        t = a.table
        label = a.id if item is None else f"{a.id} [{item}]"
        args = [self.subst(x, item) for x in t["via"]]
        is_build = args[0] in BUILD_COMMANDS
        if is_build and "--threads" not in args:
            args += ["--threads", "8"]
        fail_on = {args[i + 1] for i, x in enumerate(args[:-1]) if x == "--fail-on"}
        diag_file = self.work / ".diag" / f"{run_id}.json"
        env = self.env({k: self.subst(v, item) for k, v in t.get("env", {}).items()})
        out = self.work / t["out"] if "out" in t else None

        before_all = _snapshot(self.work, {diag_file})
        untouched = {p: _snapshot(self.work / p, set()) for p in t.get("untouched", [])}
        holding = self.work / t["holding"] if "holding" in t else None
        try:
            proc = self.run_process(args + ["--diagnostics", str(diag_file)], self.work, env, holding)
        except subprocess.TimeoutExpired:
            self.fail(label, "U1", f"exit {t['exit']}", f"no exit within {self.s.timeout:.0f} s")
            raise _Stop
        except OSError as e:
            self.fail(label, "U1", f"exit {t['exit']}", f"the binary did not start: {e}")
            raise _Stop
        code = proc.returncode
        stderr_tail = proc.stderr.decode("utf-8", "replace").strip().splitlines()[-3:]
        start = len(self.failures)

        # U1: the exit status.
        allowed = t["exit"] if isinstance(t["exit"], list) else [t["exit"]]
        if code not in allowed:
            self.fail(label, "U1", f"exit {canon.show(t['exit'])}",
                      f"exit {code}" + (f"; stderr: {' | '.join(stderr_tail)}" if stderr_tail else ""))

        # 3.5: the --diagnostics document, on every exit status of diagnostics.md 7 (0 to 3). A
        # process a test switch stopped (an expected status above 3, build.md case 24's 9) writes
        # none, by design.
        doc: diags.DiagDoc | None = None
        stopped = all(e > 3 for e in allowed) and code in allowed
        if not diag_file.is_file():
            if not stopped:
                self.fail(label, "3.5", "the --diagnostics document written", "no file")
        else:
            try:
                doc = diags.read_doc(diag_file.read_bytes())
            except (ValueError, UnicodeDecodeError) as e:
                self.fail(label, "3.5", "a diagnostics document (diagnostics.md 7)", str(e))
        if doc is not None:
            if doc.status != code:
                self.fail(label, "3.5.status", f"status {code}, the exit status", f"status {canon.show(doc.status)}")
            for p in diags.check_u2(doc.records, self.registry, fail_on):
                self.fail(label, "U2", "a registered, complete diagnostic", p)
            for cid, exp, got in diags.match(t.get("diag", {}), doc, t.get("diagnostics", "exact")):
                self.fail(label, cid, exp, got)

        # U3: exit 2 changes nothing.
        if code == 2:
            changed = _diff_snapshots(before_all, _snapshot(self.work, {diag_file}))
            if changed:
                self.fail(label, "U3", "nothing under the working folder changed by exit 2", "; ".join(changed[:5]))

        for p, snap in untouched.items():
            changed = _diff_snapshots(snap, _snapshot(self.work / p, set()))
            if changed:
                self.fail(label, "untouched", f"{p} untouched", "; ".join(changed[:5]))

        # `out`: the binary wrote there (3.3).
        if is_build and out is not None and code != 2 and not stopped:
            run_codes = {r.get("code") for r in doc.run} if doc else set()
            if not (run_codes & NO_DIRECTORY_CODES) and not (out / "diagnostics.json").is_file():
                self.fail(label, "out", f"{t['out']}/diagnostics.json written", "absent")

        if "pids_gone" in t:
            pid_file = self.work / t["pids_gone"]
            if not pid_file.is_file():
                self.fail(label, "pids_gone", f"{t['pids_gone']} written by the step fixture", "absent")
            else:
                alive = _alive_pids(pid_file)
                if alive:
                    self.fail(label, "pids_gone", "no listed process alive", f"alive: {alive}")

        for cid, ft in t.get("file", {}).items():
            for check, exp, got in files.run_file_check(cid, ft, self.work, self.case.folder, proc.stdout):
                self.fail(label, check, exp, got)

        ran = self._ran_set(out) if (is_build and out is not None) else None
        if "nodes" in t:
            self._nodes(label, t["nodes"], out)

        for cid, st in t.get("same", {}).items():
            self._same(label, cid, st, out, code, proc.stdout, ran)

        u5 = lambda: self._u5(out, diag_file)  # noqa: E731
        for cid, ct in t.get("check", {}).items():
            if item is not None:
                ct = {k: (v.replace("{item}", item) if isinstance(v, str) else v) for k, v in ct.items()}
            ctx = {"u5": u5, "stdout": proc.stdout, "note": self.notes.append}
            for check, exp, got in named.run_named(cid, ct, self.work, ctx):
                self.fail(label, check, exp, got)

        # U4: via check passes on every build that exits 0.
        if is_build and code == 0 and out is not None:
            check_diag = self.work / ".diag" / f"{run_id}.check.json"
            cp = self.run_process(["check", t["out"], "--diagnostics", str(check_diag)], self.work, env)
            if cp.returncode != 0:
                tail = cp.stderr.decode("utf-8", "replace").strip().splitlines()[-3:]
                self.fail(label, "U4", f"via check {t['out']} exits 0", f"exit {cp.returncode}: {' | '.join(tail)}")

        # U5: nothing absolute.
        for p in u5():
            self.fail(label, "U5", "no absolute path, user or host name", p)

        # U6: clean equals incremental, 1 thread equals 8.
        if is_build and out is not None and "no_rerun" not in t:
            self._rerun(label, run_id, args, env, t["out"], code)

        self._keep_result(a.id, out, code, proc.stdout, ran)
        self.record[run_id] = self._record_entry(out, code)
        if len(self.failures) > start:
            raise _Stop

    def _u5(self, out: Path | None, diag_file: Path) -> list[str]:
        targets: list[tuple[Path, str]] = []
        if out is not None:
            rel = out.relative_to(self.work).as_posix()
            targets.append((out, rel))
            targets.append((out.parent / (out.name + ".run.json"), rel + ".run.json"))
        targets.append((diag_file, f".diag/{diag_file.name}"))
        return leaks.scan(targets, leaks.Secrets.for_run(self.work, self.user))

    # --- run report -------------------------------------------------------------------------

    def _run_report(self, out: Path | None) -> dict[str, Any] | None:
        if out is None:
            return None
        p = out.parent / (out.name + ".run.json")
        if not p.is_file():
            return None
        try:
            doc = canon.loads(p.read_bytes())
        except (ValueError, UnicodeDecodeError):
            return None
        return doc if isinstance(doc, dict) else None

    def _ran_set(self, out: Path | None) -> set[str] | None:
        report = self._run_report(out)
        if report is None or not isinstance(report.get("nodes"), list):
            return None
        return {n["id"] for n in report["nodes"] if isinstance(n, dict) and n.get("status") == "ran"}

    def _nodes(self, label: str, t: dict[str, Any], out: Path | None) -> None:
        report = self._run_report(out)
        if report is None or not isinstance(report.get("nodes"), list):
            self.fail(label, "nodes", f"a run report beside {out.name if out else 'the build'}", "none readable")
            return
        status = {n.get("id"): n.get("status") for n in report["nodes"] if isinstance(n, dict)}
        ran = {i for i, s in status.items() if s == "ran"}

        def matches(entry: str, node: str) -> bool:
            return node.startswith(entry[:-1]) if entry.endswith("*") else node == entry

        want = t.get("ran")
        if "ran_file" in t:
            path = self.case.folder / t["ran_file"]
            if not path.is_file():
                self.fail(label, "nodes.ran_file", f"{t['ran_file']} in the case folder", "absent")
                want = None
            else:
                want = [x for x in path.read_text(encoding="utf-8").splitlines() if x.strip()]
        if want is not None:
            unmatched_nodes = sorted(n for n in ran if not any(matches(e, n) for e in want))
            unmatched_entries = [e for e in want if not any(matches(e, n) for n in ran)]
            if unmatched_nodes or unmatched_entries:
                self.fail(label, "nodes.ran", canon.show(want),
                          f"ran {canon.show(sorted(ran))}; extra {unmatched_nodes[:5]}, missing {unmatched_entries[:5]}")
        for e in t.get("ran_includes", []):
            if not any(matches(e, n) for n in ran):
                self.fail(label, "nodes.ran_includes", f"{e} ran", f"ran {canon.show(sorted(ran), 400)}")
        for e in t.get("reused", []):
            hits = [n for n in status if matches(e, n)]
            if not hits or any(status[n] != "reused" for n in hits):
                got = {n: status[n] for n in hits} if hits else "no such node"
                self.fail(label, "nodes.reused", f"{e} reused", canon.show(got))
        for e in t.get("not_ran", []):
            hits = sorted(n for n in ran if matches(e, n))
            if hits:
                self.fail(label, "nodes.not_ran", f"{e} did not run", f"ran: {hits[:5]}")

    # --- same -------------------------------------------------------------------------------

    def _manifest(self, out: Path | None) -> dict[str, Any] | None:
        if out is None or not (out / "manifest.json").is_file():
            return None
        try:
            m = canon.loads((out / "manifest.json").read_bytes())
        except (ValueError, UnicodeDecodeError):
            return None
        return m if isinstance(m, dict) else None

    def _files_digest(self, out: Path | None) -> dict[str, str] | None:
        m = self._manifest(out)
        if m is None or not isinstance(m.get("files"), dict):
            return None
        result: dict[str, str] = {}
        for p in m["files"]:
            f = out / p
            result[p] = _sha(f.read_bytes()) if f.is_file() else "absent"
        return result

    def _capture(self, path: Path) -> Any:
        if path.is_file():
            return ("file", path.read_bytes())
        if path.is_dir():
            return ("dir", {k: v[3] if v[0] == "file" else v[0] for k, v in _snapshot(path, set()).items()})
        return ("absent",)

    def _keep_result(self, aid: str, out: Path | None, code: int, stdout: bytes, ran: set[str] | None) -> None:
        m = self._manifest(out)
        diag_json = out / "diagnostics.json" if out is not None else None
        self.results[aid] = {
            "exit": code,
            "stdout": ("file", stdout),
            "id": m.get("id") if m else None,
            "files": self._files_digest(out),
            "diagnostics": self._capture(diag_json) if diag_json else ("absent",),
            "ran": ran,
            "paths": {p: self._capture(self.work / p) for p in self.same_paths.get(aid, ())},
        }

    def _same(self, label: str, cid: str, st: dict[str, Any], out: Path | None, code: int,
              stdout: bytes, ran: set[str] | None) -> None:
        check = f"same.{cid}"
        earlier = self.results.get(st["action"])
        if earlier is None:
            self.fail(label, check, f"action {st['action']} ran", "it has no result")
            return
        what = st["what"]
        if what == "id":
            m = self._manifest(out)
            a, b = earlier["id"], (m.get("id") if m else None)
            if a is None or b is None:
                self.fail(label, check, "a build id on both sides", f"{canon.show(a)} and {canon.show(b)}")
                return
        elif what == "files":
            a, b = earlier["files"], self._files_digest(out)
            if a is None or b is None:
                self.fail(label, check, "a manifest's files on both sides", "a manifest is missing")
                return
        elif what == "ran":
            a, b = earlier["ran"], ran
            if a is None or b is None:
                self.fail(label, check, "a run report on both sides", "a run report is missing")
                return
        elif what == "stdout":
            a, b = earlier["stdout"], ("file", stdout)
        elif what == "diagnostics":
            a = earlier["diagnostics"]
            b = self._capture(out / "diagnostics.json") if out is not None else ("absent",)
        else:
            a, b = earlier["paths"].get(what, ("absent",)), self._capture(self.work / what)
        if "ignore" in st and isinstance(a, tuple) and isinstance(b, tuple) and a[0] == b[0] == "file":
            try:
                ja, jb = canon.loads(a[1]), canon.loads(b[1])
            except (ValueError, UnicodeDecodeError) as e:
                self.fail(label, check, "JSON on both sides for `ignore`", str(e))
                return
            for p in st["ignore"]:
                canon.remove(ja, p)
                canon.remove(jb, p)
            a, b = canon.encode(ja), canon.encode(jb)
        equal = a == b
        if st.get("differ", False):
            if equal:
                self.fail(label, check, f"{what} differs from action {st['action']}", "the same")
        elif not equal:
            self.fail(label, check, f"{what} the same as action {st['action']}", _describe_diff(a, b))

    # --- U6 ---------------------------------------------------------------------------------

    def _input_copy(self, dest: Path, args: list[str], own_out: str) -> None:
        """Copy the working folder's inputs as they stand: not build directories, run reports or
        the store; a build directory the action reads (named by --against, or by a world's
        `file`) is an input and is copied (conformance.md 5.1)."""
        read_dirs: set[str] = set()
        for i, x in enumerate(args[:-1]):
            if x == "--against":
                read_dirs.add(Path(args[i + 1]).as_posix().rstrip("/"))
        texts = ""
        for p in self.work.rglob("*.toml"):
            parts = set(p.relative_to(self.work).parts)
            if parts & {".via", ".config", ".home", ".diag"}:
                continue
            try:
                texts += p.read_text(encoding="utf-8", errors="replace")
            except OSError:
                pass
        for o in self.outs:
            if o != own_out and f"{o}/" in texts:
                read_dirs.add(o)
        store_args = [args[i + 1] for i, x in enumerate(args[:-1]) if x == "--store"]
        skip: set[str] = {".config", ".home", ".diag"}
        for o in self.outs:
            if o in read_dirs:
                continue
            for suffix in ("", ".run.json", ".lock", ".partial", ".old", ".repro"):
                skip.add(o + suffix)
        for s in store_args:
            skip.add(Path(s).as_posix().rstrip("/"))
        dest.mkdir(parents=True)

        def ignore(dirpath: str, names: list[str]) -> list[str]:
            rel = Path(dirpath).relative_to(self.work)
            out = []
            for n in names:
                r = (rel / n).as_posix()
                if r in skip or n == ".via":
                    out.append(n)
            return out

        shutil.copytree(self.work, dest, symlinks=True, dirs_exist_ok=True, ignore=ignore)

    def _build_in(self, folder: Path, args: list[str], env: dict[str, str], threads: int, tag: str):
        a = list(args)
        i = a.index("--threads")
        a[i + 1] = str(threads)
        diag = self.work / ".diag" / f"{tag}.json"
        return self.run_process(a + ["--diagnostics", str(diag)], folder, env)

    def _compare_builds(self, a_dir: Path, a_code: int, b_dir: Path, b_code: int, out: str) -> list[str]:
        diffs: list[str] = []
        if a_code != b_code:
            diffs.append(f"exit {a_code} and {b_code}")
        da, db = a_dir / out / "diagnostics.json", b_dir / out / "diagnostics.json"
        ca = da.read_bytes() if da.is_file() else None
        cb = db.read_bytes() if db.is_file() else None
        if ca != cb:
            diffs.append("diagnostics.json differs")
        if a_code == 0 and b_code == 0:
            ma, mb = self._manifest(a_dir / out), self._manifest(b_dir / out)
            if ma is None or mb is None:
                diffs.append("a manifest is missing")
            else:
                if ma.get("id") != mb.get("id"):
                    diffs.append(f"id {ma.get('id')} and {mb.get('id')}")
                fa, fb = ma.get("files") or {}, mb.get("files") or {}
                if sorted(fa) != sorted(fb):
                    diffs.append("the manifests list different files")
                for p in sorted(set(fa) & set(fb), key=str.encode):
                    pa, pb = a_dir / out / p, b_dir / out / p
                    if not (pa.is_file() and pb.is_file() and pa.read_bytes() == pb.read_bytes()):
                        diffs.append(f"{p} differs")
                        if len(diffs) > 6:
                            break
        return diffs

    def _rerun(self, label: str, run_id: str, args: list[str], env: dict[str, str], out: str, code: int) -> None:
        clean1 = self.work.parent / f"{self.work.name}.clean1-{run_id}"
        self.siblings.append(clean1)
        self._input_copy(clean1, args, out)
        r1 = self._build_in(clean1, args, env, 1, f"{run_id}.clean1")
        diffs = self._compare_builds(self.work, code, clean1, r1.returncode, out)
        if not diffs:
            return
        clean8 = self.work.parent / f"{self.work.name}.clean8-{run_id}"
        self.siblings.append(clean8)
        self._input_copy(clean8, args, out)
        r8 = self._build_in(clean8, args, env, 8, f"{run_id}.clean8")
        incremental_ok = not self._compare_builds(self.work, code, clean8, r8.returncode, out)
        threads_ok = not self._compare_builds(clean8, r8.returncode, clean1, r1.returncode, out)
        if incremental_ok and not threads_ok:
            which = "1 thread differs from 8 (clean at 8 equals the action)"
        elif threads_ok and not incremental_ok:
            which = "clean differs from incremental (clean at 1 and at 8 agree)"
        else:
            which = "clean differs from incremental, and 1 thread from 8"
        self.fail(label, "U6", "a clean build at 1 thread equal to the action's", f"{which}: {'; '.join(diffs[:5])}")

    # --- U7 record --------------------------------------------------------------------------

    def _record_entry(self, out: Path | None, code: int) -> dict[str, Any]:
        entry: dict[str, Any] = {"exit": code, "id": None, "files": {}, "diagnostics": []}
        m = self._manifest(out)
        if m is not None:
            entry["id"] = m.get("id")
            fs = m.get("files")
            if isinstance(fs, dict):
                entry["files"] = {p: (v.get("digest") if isinstance(v, dict) else None) for p, v in fs.items()}
        if out is not None and (out / "diagnostics.json").is_file():
            try:
                d = canon.loads((out / "diagnostics.json").read_bytes())
                recs = d.get("diagnostics", []) if isinstance(d, dict) else []
            except (ValueError, UnicodeDecodeError):
                recs = []
            cleaned = []
            for r in recs:
                if not isinstance(r, dict):
                    continue
                r = {k: v for k, v in r.items() if k not in ("message", "hint")}
                cleaned.append(r)
            entry["diagnostics"] = cleaned
        return entry

    # --- the whole case ---------------------------------------------------------------------

    def run(self) -> Outcome:
        name = self.case.name
        try:
            self.prepare()
            for a in self.case.actions:
                if a.kind == "via":
                    self.via_action(a)
                else:
                    self.edit(a)
        except CaseError as e:
            self.cleanup(keep=True)
            return Outcome(name, "error", [str(e), f"kept {self.work}"], self.work, self.case.milestone)
        except _Stop:
            pass
        if self.failures:
            detail = [f"action {a}, check {c}: expected {e}; got {g}" for a, c, e, g in self.failures]
            self.cleanup(keep=True)
            detail.append(f"kept {self.work}")
            return Outcome(name, "fail", detail, self.work, self.case.milestone, self._rec())
        self.cleanup(keep=False)
        return Outcome(name, "pass", self.notes, None, self.case.milestone, self._rec())

    def _rec(self) -> dict[str, Any]:
        return {"one_system": self.case.one_system, "actions": self.record}

    def cleanup(self, keep: bool) -> None:
        """A failed case's folders are kept for inspection (6); a passing one's are deleted."""
        if keep:
            return
        for p in [self.work] + self.siblings:
            shutil.rmtree(p, ignore_errors=True)


def _describe_diff(a: Any, b: Any) -> str:
    """Say how two captured values differ, briefly: which members, or how many bytes."""
    if isinstance(a, dict) and isinstance(b, dict):
        keys = sorted(set(a) | set(b), key=str.encode)
        bad = [k for k in keys if a.get(k) != b.get(k)]
        return f"{len(bad)} differ, first {bad[:3]}"
    if isinstance(a, set) and isinstance(b, set):
        return f"only before {sorted(a - b)[:5]}, only after {sorted(b - a)[:5]}"
    if isinstance(a, tuple) and isinstance(b, tuple):
        if a[0] != b[0]:
            return f"{a[0]} and {b[0]}"
        if a[0] == "file":
            return f"{len(a[1])} and {len(b[1])} bytes that differ"
        if a[0] == "dir":
            return _describe_diff(a[1], b[1])
    return f"{a!r:.200} and {b!r:.200}"


def _hold(path: Path):
    """An operating-system advisory lock held by the runner (graph.md's `busy` case)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    fh = open(path, "a+b")
    if sys.platform == "win32":
        import msvcrt

        fh.seek(0)
        msvcrt.locking(fh.fileno(), msvcrt.LK_NBLCK, 1)
    else:
        import fcntl

        fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    return fh


def _release(fh) -> None:
    try:
        if sys.platform == "win32":
            import msvcrt

            fh.seek(0)
            msvcrt.locking(fh.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            import fcntl

            fcntl.flock(fh.fileno(), fcntl.LOCK_UN)
    finally:
        fh.close()


def _run_bounded(argv: list[str], cwd: Path, env: dict[str, str], timeout: float) -> subprocess.CompletedProcess:
    """Run `via` with a time limit that holds even when it hangs.

    Output goes to files, not pipes: a step worker that inherited a pipe would keep it open after
    `via` is killed, and reading it would block for ever. `via` starts its own process group (a
    Job Object on Windows) so that on a timeout its workers die with it rather than outlive the
    case U1 reports.
    """
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        kwargs: dict[str, Any] = {}
        if sys.platform == "win32":
            kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
        else:
            kwargs["start_new_session"] = True
        proc = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=out, stderr=err, **kwargs)
        job = _job_for(proc) if sys.platform == "win32" else None
        try:
            try:
                code = proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                _kill_tree(proc, job)
                try:
                    proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    pass
                raise
        finally:
            if job is not None:
                _close_handle(job)
        out.seek(0)
        err.seek(0)
        return subprocess.CompletedProcess(argv, code, out.read(), err.read())


def _kill_tree(proc: subprocess.Popen, job: Any) -> None:
    if sys.platform == "win32":
        if job is not None:
            _kernel32().TerminateJobObject(job, 1)
        proc.kill()
        return
    import signal

    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        proc.kill()


def _kernel32():
    """kernel32 with the signatures this module calls: without restype a HANDLE is cut to a C int
    on 64-bit Windows."""
    import ctypes
    from ctypes import wintypes

    k = ctypes.WinDLL("kernel32", use_last_error=True)
    k.OpenProcess.restype = wintypes.HANDLE
    k.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    k.CloseHandle.restype = wintypes.BOOL
    k.CloseHandle.argtypes = [wintypes.HANDLE]
    k.GetExitCodeProcess.restype = wintypes.BOOL
    k.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    k.GetProcessTimes.restype = wintypes.BOOL
    k.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
    k.CreateJobObjectW.restype = wintypes.HANDLE
    k.CreateJobObjectW.argtypes = [wintypes.LPVOID, wintypes.LPCWSTR]
    k.AssignProcessToJobObject.restype = wintypes.BOOL
    k.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    k.TerminateJobObject.restype = wintypes.BOOL
    k.TerminateJobObject.argtypes = [wintypes.HANDLE, wintypes.UINT]
    return k


def _job_for(proc: subprocess.Popen) -> Any:
    """A Job Object holding `via`; its children join it as they start. None if it cannot be made,
    and a timeout then kills `via` alone."""
    k = _kernel32()
    job = k.CreateJobObjectW(None, None)
    if not job:
        return None
    if not k.AssignProcessToJobObject(job, int(proc._handle)):  # type: ignore[attr-defined]
        k.CloseHandle(job)
        return None
    return job


def _close_handle(handle: Any) -> None:
    _kernel32().CloseHandle(handle)


def _pid_alive(pid: int, since: float) -> bool:
    """Whether the process `pid` runs and started no later than `since` (a POSIX time): a process
    started after the pid file was written has a reused pid and is not the one listed."""
    if sys.platform == "win32":
        import ctypes
        from ctypes import wintypes

        k = _kernel32()
        handle = k.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
        if not handle:
            return False
        try:
            code = wintypes.DWORD()
            if not k.GetExitCodeProcess(handle, ctypes.byref(code)) or code.value != 259:  # STILL_ACTIVE
                return False
            times = [wintypes.FILETIME() for _ in range(4)]
            if k.GetProcessTimes(handle, *(ctypes.byref(t) for t in times)):
                ticks = (times[0].dwHighDateTime << 32) | times[0].dwLowDateTime
                created = ticks / 1e7 - 11644473600.0  # 100 ns ticks since 1601 to POSIX seconds
                if created > since + 1.0:
                    return False
            return True
        finally:
            k.CloseHandle(handle)
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _alive_pids(path: Path) -> list[int]:
    """Processes of the file still alive after a short grace: the engine kills a step's process
    tree as it exits, and the kernel may take a moment to reap it."""
    pids = [int(x) for x in path.read_text(encoding="utf-8").split() if x.strip().isdigit()]
    since = path.stat().st_mtime
    deadline = time.monotonic() + 2.0
    while True:
        alive = [p for p in pids if _pid_alive(p, since)]
        if not alive or time.monotonic() > deadline:
            return alive
        time.sleep(0.05)


def run_case(case: Case, settings: Settings, registry: diags.Registry) -> Outcome:
    if case.pending is not None:
        return Outcome(case.name, "pending", [case.pending], milestone=case.milestone)
    if case.milestone > settings.milestone:
        return Outcome(case.name, "pending", [f"M{case.milestone}"], milestone=case.milestone)
    if not (settings.repo / "spec" / f"{case.chapter}.md").is_file():
        return Outcome(case.name, "error", [f"spec/{case.chapter}.md is not a chapter"], milestone=case.milestone)
    return CaseRun(case, settings, registry).run()
