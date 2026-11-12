"""A fake `via` for the runner's self-tests: small, honest by default, and wrong on request.

It speaks just enough of the command line (build.md 14) for the runner to be tested end to end:
`--version`, `help`, `config program`, `trust`, `build`, `check`, and `cli.usage` for anything
else. A world is TOML with constant fields (`[field.<n>] value = 1.5`), point features and
instances (`at = [x, y]`), and `[[fake.diag]]` records to raise. Digests are SHA-256 here; the real
binary's are BLAKE3 (build.md 3.3), which the runner never computes.

The environment variable FAKE_VIA, a comma-separated list, switches on one defect each:

    threads      a build file records the thread count (1 thread differs from 8)
    incremental  a build file records how many nodes were reused (clean differs from incremental)
    abspath      a build file holds the working folder's absolute path
    nodiag       the --diagnostics document is not written
    status       the --diagnostics document's status is wrong
    touch2       a usage error (exit 2) leaves a file behind
    checkfail    `via check` exits 1
    rewrite      an unchanged rebuild rewrites the directory anyway
    hang         start a worker that inherits stdout and stderr, write its pid to hang.pid, and
                 never exit (the runner's timeout must kill both and not block on the pipes)
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from via_conformance import canon  # noqa: E402

BUGS = set(filter(None, os.environ.get("FAKE_VIA", "").split(",")))
RUN_CODES = {"identity.no_previous"}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def cj(value) -> bytes:
    return canon.canonical_bytes(value)


def merge(base: dict, over: dict) -> dict:
    out = dict(base)
    for k, v in over.items():
        if isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = merge(out[k], v)
        else:
            out[k] = v
    return out


def doc(records: list[dict], status: int) -> dict:
    counts = {"error": 0, "failed_on": 0, "info": 0, "warning": 0}
    for r in records:
        if r.get("severity") in counts:
            counts[r["severity"]] += 1
    return {"counts": counts, "diagnostics": records, "format": "via-diagnostics-1", "lints": [], "status": status}


class Cli:
    def __init__(self, argv: list[str]):
        self.diag_path: Path | None = None
        args = []
        i = 0
        while i < len(argv):
            if argv[i] == "--diagnostics" and i + 1 < len(argv):
                self.diag_path = Path(argv[i + 1])
                i += 2
                continue
            args.append(argv[i])
            i += 1
        self.args = args

    def finish(self, records: list[dict], status: int) -> int:
        for r in records:
            print(f"{r.get('severity')}[{r.get('code')}] {r.get('subject')}: {r.get('message', '')}", file=sys.stderr)
        if self.diag_path is not None and "nodiag" not in BUGS:
            self.diag_path.parent.mkdir(parents=True, exist_ok=True)
            self.diag_path.write_bytes(cj(doc(records, status + (1 if "status" in BUGS else 0))))
        return status

    def usage(self, index: int, what: str) -> int:
        if "touch2" in BUGS:
            Path("oops.txt").write_bytes(("left behind\n").encode("utf-8"))
        rec = {"argument": index, "code": "cli.usage", "hint": "run `via help`", "message": what,
               "severity": "error", "subject": "cli"}
        return self.finish([rec], 2)

    def main(self) -> int:
        a = self.args
        if "hang" in BUGS:
            worker = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(120)"])
            Path("hang.pid").write_bytes(f"{worker.pid}\n".encode("utf-8"))
            time.sleep(120)
        if a == ["--version"]:
            print("via 0.1.0")
            return self.finish([], 0)
        if a == ["help"]:
            print("usage: via <command> ...")
            return self.finish([], 0)
        if len(a) == 4 and a[:2] == ["config", "program"]:
            cfg = Path(os.environ["VIA_CONFIG_DIR"])
            cfg.mkdir(parents=True, exist_ok=True)
            (cfg / "config.toml").write_bytes((f'[program.{a[2]}]\npath = "{a[3]}"\n').encode("utf-8"))
            return self.finish([], 0)
        if a and a[0] == "trust":
            cfg = Path(os.environ["VIA_CONFIG_DIR"])
            cfg.mkdir(parents=True, exist_ok=True)
            (cfg / "trust.toml").write_bytes(("# trusted\n").encode("utf-8"))
            return self.finish([], 0)
        if a and a[0] == "build":
            return self.build(a[1:])
        if len(a) == 2 and a[0] == "check":
            return self.check(Path(a[1]))
        return self.usage(1 if not a else len(a), f"unknown arguments {a}")

    # --- build ----------------------------------------------------------------------------

    def build(self, a: list[str]) -> int:
        world_arg = None
        out = None
        threads = 0
        changes: list[str] = []
        against = None
        fail_on: list[str] = []
        i = 0
        while i < len(a):
            x = a[i]
            if x in ("--out", "--threads", "--change", "--against", "--store", "--fail-on", "--lib"):
                if i + 1 >= len(a):
                    return self.usage(i + 2, f"{x} needs a value")
                v = a[i + 1]
                if x == "--out":
                    out = Path(v)
                elif x == "--threads":
                    threads = int(v)
                elif x == "--change":
                    changes.append(v)
                elif x == "--against":
                    against = Path(v)
                elif x == "--fail-on":
                    fail_on.append(v)
                i += 2
                continue
            if x.startswith("--"):
                return self.usage(i + 2, f"unknown flag {x}")
            world_arg = Path(x)
            i += 1
        if world_arg is None:
            return self.usage(1, "no world")
        text = world_arg.read_bytes()
        world = tomllib.loads(text.decode("utf-8"))
        for c in changes:
            if "/" in c or "." in c:
                world = merge(world, tomllib.loads((world_arg.parent / c).read_text(encoding="utf-8")))
            else:
                world = merge(world, world.get("change", {}).get(c, {}))
        name = world.get("name", world_arg.stem)
        if out is None:
            out = world_arg.parent / "build" / name
        store = world_arg.parent / ".via" / "store"
        store.mkdir(parents=True, exist_ok=True)
        index_path = store / "index.json"
        index = json.loads(index_path.read_text(encoding="utf-8")) if index_path.exists() else {}

        world_recs = []
        for r in world.get("fake", {}).get("diag", []):
            rec = {"message": "fake"} | r
            if rec.get("code") in fail_on and rec.get("severity") == "warning":
                rec["failed_on"] = True
            world_recs.append(rec)
        has_error = any(r.get("severity") == "error" or r.get("failed_on") for r in world_recs)

        fields = {n: float(f.get("value", 0)) for n, f in sorted(world.get("field", {}).items())}
        nodes = []
        reused = 0
        merged_key = sha(cj({k: v for k, v in world.items() if k != "change"}))
        for nid, key in [("merge", merged_key)] + [(f"field:{n}/v:build", sha(f"{n}={v}".encode())) for n, v in fields.items()]:
            status = "reused" if key in index else "ran"
            reused += status == "reused"
            nodes.append({"cause": None if status == "reused" else {"reason": "no record"}, "family": nid.split(":")[0],
                          "failed": False, "id": nid, "key": key, "ms": 0, "needed_by": None,
                          "status": status, "value": key})
            index[key] = True
        index_path.write_bytes((json.dumps(index, sort_keys=True)).encode("utf-8"))

        previous_path = out / ".previous"
        previous = json.loads(previous_path.read_text(encoding="utf-8")) if previous_path.exists() else {"id": None}
        run_recs = []
        if against is None and previous.get("id") is None:
            run_recs.append({"code": "identity.no_previous", "file": world_arg.name, "line": 1, "column": 1,
                             "bytes": [0, 0], "hint": "none needed", "message": "no baseline",
                             "severity": "info", "subject": "world"})

        diag_bytes = cj(doc(world_recs, 1 if has_error else 0))
        exit_code = 1 if has_error else 0
        run_key = sha(text + cj(changes))
        report = {"build": {"complete": True, "id": None, "key": run_key, "null": False, "previous": None},
                  "diagnostics": run_recs, "engine": {"epoch": 1, "version": "0.1.0"}, "format": "via-run-0",
                  "nodes": nodes, "steps": [], "system": {"arch": "x", "os": "y"}, "threads": threads,
                  "totals": {"ms": 0, "nodes": len(nodes), "ran": len(nodes) - reused, "recomputed": 0,
                             "reused": reused, "skipped": 0}}

        if has_error:
            if out.exists():
                shutil.rmtree(out)
            out.mkdir(parents=True)
            (out / "diagnostics.json").write_bytes(diag_bytes)
            (out / ".previous").write_bytes(cj(previous | {"format": "via-previous-0"}))
            Path(str(out) + ".run.json").write_bytes(cj(report))
            return self.finish(world_recs + run_recs, exit_code)

        files: dict[str, bytes] = {}
        for n, v in fields.items():
            tmp = Path(str(out) + f".{n}.tmp.npy")
            tmp.parent.mkdir(parents=True, exist_ok=True)
            np.save(tmp, np.full((3, 3), v, dtype="<f4"))
            files[f"fields/{n}.npy"] = tmp.read_bytes()
            tmp.unlink()
        feats = [{"at": list(f["at"]) + [0], "kind": None, "name": n}
                 for n, f in sorted(world.get("feature", {}).items())]
        extra: dict = {}
        if "threads" in BUGS:
            extra["threads"] = threads
        if "incremental" in BUGS:
            extra["reused"] = reused
        if "abspath" in BUGS:
            extra["where"] = str(Path.cwd())
        files["features.json"] = cj({"features": feats, "footprints": []} | extra)
        inst = sorted(world.get("instance", {}).items())
        xs = np.array([round(i["at"][0] * 1000) for _, i in inst], dtype="<i4")
        ys = np.array([round(i["at"][1] * 1000) for _, i in inst], dtype="<i4")
        for col, arr in (("x", xs), ("y", ys)):
            tmp = Path(str(out) + f".{col}.tmp.npy")
            np.save(tmp, arr)
            files[f"instances/{col}.npy"] = tmp.read_bytes()
            tmp.unlink()
        files["instances/names.txt"] = "".join(n + "\n" for n, _ in inst).encode()

        manifest = {"engine": {"epoch": 1, "name": "via"}, "files": {p: {"bytes": len(b), "digest": sha(b)}
                    for p, b in sorted(files.items())}, "format": "via-build-0", "world": name}
        bid = sha(cj(manifest))

        # The change report, against --against or the directory's last complete build.
        base_dir = against if against is not None else (out if previous.get("id") else None)
        entries = []
        against_obj = None
        if base_dir is not None and (base_dir / "manifest.json").exists():
            bm = json.loads((base_dir / "manifest.json").read_text(encoding="utf-8"))
            against_obj = {"id": bm["id"], "key": bm["run"]["key"]}
            for p in sorted(set(bm["files"]) | set(manifest["files"])):
                if bm["files"].get(p, {}).get("digest") != manifest["files"].get(p, {}).get("digest"):
                    entries.append({"name": p, "table": "field", "what": ["changed"]})
        changes_bytes = cj({"against": against_obj, "entries": entries, "format": "via-changes-0", "summary": []})
        manifest |= {"id": bid, "locations": {"pack_root": None},
                     "run": {"changes": sorted(changes), "flags": {"fail_on": sorted(fail_on), "stand_ins": False},
                             "inputs": [], "key": run_key, "version": "0.1.0",
                             "reports": {"changes.json": {"bytes": len(changes_bytes), "digest": sha(changes_bytes)},
                                         "diagnostics.json": {"bytes": len(diag_bytes), "digest": sha(diag_bytes)}}}}
        report["build"]["id"] = bid

        unchanged = (
            against is None
            and (out / "manifest.json").exists()
            and json.loads((out / "manifest.json").read_text(encoding="utf-8")).get("run", {}).get("key") == run_key
            and "rewrite" not in BUGS
        )
        if not unchanged:
            if out.exists():
                shutil.rmtree(out)
            for p, b in files.items():
                (out / p).parent.mkdir(parents=True, exist_ok=True)
                (out / p).write_bytes(b)
            (out / "manifest.json").write_bytes(cj(manifest))
            (out / "diagnostics.json").write_bytes(diag_bytes)
            (out / "changes.json").write_bytes(changes_bytes)
            (out / ".previous").write_bytes(cj({"format": "via-previous-0", "id": bid, "manifest": sha(cj(manifest))}))
        Path(str(out) + ".run.json").write_bytes(cj(report))
        return self.finish(world_recs + run_recs, exit_code)

    def check(self, d: Path) -> int:
        bad = []
        m_path = d / "manifest.json"
        if "checkfail" in BUGS or not m_path.exists():
            bad.append("manifest")
        else:
            m = json.loads(m_path.read_text(encoding="utf-8"))
            body = {k: v for k, v in m.items() if k not in ("id", "run", "locations")}
            if sha(cj(body)) != m.get("id"):
                bad.append("id")
            for p, rec in m["files"].items():
                f = d / p
                if not f.exists() or sha(f.read_bytes()) != rec["digest"]:
                    bad.append(p)
        recs = [{"code": "check.digest", "file": p, "hint": "rebuild", "message": "differs",
                 "severity": "error", "subject": "world"} for p in bad]
        return self.finish(recs, 1 if bad else 0)


if __name__ == "__main__":
    # A real binary writes `\n`; Python's text streams would write `\r\n` on Windows.
    sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    sys.stderr.reconfigure(encoding="utf-8", newline="\n")
    sys.exit(Cli(sys.argv[1:]).main())
