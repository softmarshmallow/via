"""Reading `case.toml` (conformance.md 3).

Every key is checked here, before anything runs: an unknown key, a missing required key or a
wrong type is a case error, never ignored (3), and a pending case is parsed too, so a malformed
one is an error now, not when its milestone comes (4).
"""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import Any

FORMAT = "via-case-1"
ID_RE = re.compile(r"^[a-z][a-z0-9_]*$")
MAX_ID_BYTES = 48

ACTION_KINDS = ("via", "replace", "write", "delete", "flip", "flip_object", "rerecord", "truncate", "derive")
DERIVE_AS = ("segments", "cells", "interpolate", "project", "params")
SAME_WHAT_WORDS = ("id", "files", "diagnostics", "stdout", "ran")
DIAG_MODES = ("exact", "includes", "any")

# conformance.md 3.4, the named-check table: fn -> (required arguments, optional arguments).
# Rows are added with the cases that need them; a name not here is a case error.
NAMED_CHECKS: dict[str, tuple[set[str], set[str]]] = {
    "weights_sum_at_most": ({"path", "limit"}, set()),
    "weights_sum_255_or_0": ({"path"}, set()),
    "no_absolute_path": (set(), set()),
    "changed_within_windows": ({"before", "after", "fields", "kinds", "world"}, set()),
    "no_origin_inside": ({"build"}, {"at", "radius", "points", "prefix"}),
    "changes_by_prefix": ({"path", "prefix", "what"}, {"names"}),
    "vectors_match": ({"path"}, {"fns"}),
    "json_count": ({"path", "pointer", "count"}, set()),
    "run_node": ({"path", "node", "includes"}, set()),
    "array_window": ({"whole", "part", "window"}, set()),
    "run_report_valid": ({"path"}, set()),
    "pack_root_resolves": ({"build"}, set()),
    "json_list": ({"path", "pointer"}, {"from", "at_least", "includes", "starts", "last", "nulls"}),
    "instance_at": ({"build", "name", "at"}, set()),
    "changes_inside": ({"path", "prefix", "at", "radius", "what", "cause"}, set()),
    "changes_absent": ({"path"}, {"entry_names", "causes"}),
    "network_valid": ({"path", "input"}, set()),
    "network_measures": ({"path", "reference"}, set()),
    "noding_valid": ({"path", "input"}, {"bound"}),
    "noding_same": ({"first", "second"}, set()),
    "names_same": ({"first", "second"}, set()),
    "cells_valid": ({"path", "input", "network"}, {"bound"}),
    "sample_terrain": ({"path", "input", "field", "manifest"}, set()),
    "course_valid": ({"path", "feature", "start", "end", "min_segment"}, {"passes", "within"}),
    "entry_causes": ({"path", "table", "prefix"}, {"named"}),
    "post_step_at_most": ({"path", "resolution", "window", "limit"}, {"origin"}),
    "roundtrip_point": ({"path", "input"}, {"bound"}),
    "roundtrip_t": ({"path", "input"}, {"bound"}),
    "stroke_measures": ({"path", "reference", "distance"}, set()),
    "id_rule": ({"build"}, set()),
    "names_count": ({"path", "prefix", "count"}, set()),
    "names_kept": ({"first", "second", "prefix"}, {"except_within"}),
}


class CaseError(Exception):
    """The case itself is malformed (conformance.md 6, outcome `error`)."""


@dataclass
class Copy:
    id: str
    src: str
    to: str


@dataclass
class Action:
    id: str
    kind: str
    table: dict[str, Any]

    @property
    def target(self) -> Any:
        return self.table[self.kind]


@dataclass
class Case:
    chapter: str
    id: str
    folder: Path
    summary: str
    milestone: int
    actions: list[Action]
    copies: list[Copy] = field(default_factory=list)
    pending: str | None = None
    one_system: str | None = None

    @property
    def name(self) -> str:
        return f"{self.chapter}/{self.id}"


def _where(*parts: str) -> str:
    return ".".join(parts)


def _expect_type(value: Any, types: tuple[type, ...], where: str, what: str) -> None:
    # A TOML boolean is a Python int subclass; never let it pass as an integer.
    if isinstance(value, bool) and bool not in types:
        raise CaseError(f"{where}: expected {what}, got a boolean")
    if not isinstance(value, types):
        raise CaseError(f"{where}: expected {what}, got {type(value).__name__}")


def _str(value: Any, where: str) -> str:
    _expect_type(value, (str,), where, "a string")
    return value


def _int(value: Any, where: str, lo: int | None = None, hi: int | None = None) -> int:
    _expect_type(value, (int,), where, "an integer")
    if lo is not None and value < lo:
        raise CaseError(f"{where}: {value} is below {lo}")
    if hi is not None and value > hi:
        raise CaseError(f"{where}: {value} is above {hi}")
    return value


def _number(value: Any, where: str) -> float | int:
    _expect_type(value, (int, float), where, "a number")
    return value


def _str_list(value: Any, where: str) -> list[str]:
    _expect_type(value, (list,), where, "a list of strings")
    for i, v in enumerate(value):
        _str(v, f"{where}[{i}]")
    return value


def _table(value: Any, where: str) -> dict[str, Any]:
    _expect_type(value, (dict,), where, "a table")
    return value


def _keys(table: dict[str, Any], where: str, required: set[str], optional: set[str]) -> None:
    unknown = sorted(set(table) - required - optional)
    if unknown:
        raise CaseError(f"{where}: unknown key {unknown[0]!r}")
    missing = sorted(required - set(table))
    if missing:
        raise CaseError(f"{where}: missing required key {missing[0]!r}")


def _relative(path: str, where: str) -> str:
    """A path relative to the working folder (or the repository): never absolute, never above."""
    if not path:
        raise CaseError(f"{where}: empty path")
    p = PurePosixPath(path.replace("\\", "/"))
    if p.is_absolute() or re.match(r"^[A-Za-z]:", path) or ".." in p.parts:
        raise CaseError(f"{where}: {path!r} must be relative and stay inside its root")
    return path


def _expected_path(path: str, where: str) -> str:
    _relative(path, where)
    if not path.startswith("expected/"):
        raise CaseError(f"{where}: {path!r} must be a file under expected/")
    return path


def _check_value_like(t: dict[str, Any], where: str) -> None:
    """A value from TOML has no null; anything TOML can hold is a JSON value but datetimes."""

    def walk(v: Any, w: str) -> None:
        if isinstance(v, dict):
            for k, x in v.items():
                walk(x, f"{w}.{k}")
        elif isinstance(v, list):
            for i, x in enumerate(v):
                walk(x, f"{w}[{i}]")
        elif not isinstance(v, (str, int, float, bool)):
            raise CaseError(f"{w}: a {type(v).__name__} cannot be compared with JSON")

    walk(t, where)


def _parse_file_check(t: dict[str, Any], where: str) -> None:
    allowed = {
        "path", "stream", "exists", "listing", "equals", "ignore", "text", "size", "pointer",
        "line", "value", "value_file", "includes", "dtype", "shape", "post", "plane", "all",
        "bits", "tolerance",
    }
    _keys(t, where, set(), allowed)
    if ("path" in t) == ("stream" in t):
        raise CaseError(f"{where}: name exactly one of `path` or `stream`")
    if "path" in t:
        _relative(_str(t["path"], f"{where}.path"), f"{where}.path")
    else:
        if _str(t["stream"], f"{where}.stream") != "stdout":
            raise CaseError(f"{where}.stream: only \"stdout\" is a stream")
        for k in ("exists", "listing", "dtype", "shape", "post", "all"):
            if k in t:
                raise CaseError(f"{where}: `{k}` does not apply to standard output")
    if "exists" in t:
        _expect_type(t["exists"], (bool,), f"{where}.exists", "a boolean")
    if "listing" in t:
        _str_list(t["listing"], f"{where}.listing")
    if "equals" in t:
        _expected_path(_str(t["equals"], f"{where}.equals"), f"{where}.equals")
    if "ignore" in t:
        if "equals" not in t:
            raise CaseError(f"{where}: `ignore` is only taken with `equals`")
        for i, p in enumerate(_str_list(t["ignore"], f"{where}.ignore")):
            if not p.startswith("/"):
                raise CaseError(f"{where}.ignore[{i}]: {p!r} is not a JSON Pointer to a member")
    if "text" in t:
        _str(t["text"], f"{where}.text")
    if "size" in t:
        _int(t["size"], f"{where}.size", 0)
    compared = [k for k in ("value", "value_file", "includes") if k in t]
    if "pointer" in t:
        p = _str(t["pointer"], f"{where}.pointer")
        if p and not p.startswith("/"):
            raise CaseError(f"{where}.pointer: {p!r} is not a JSON Pointer")
        if len(compared) != 1:
            raise CaseError(f"{where}: `pointer` takes exactly one of `value`, `value_file`, `includes`")
        if "post" in t:
            raise CaseError(f"{where}: `pointer` and `post` cannot both be given")
    if "line" in t:
        if "pointer" not in t:
            raise CaseError(f"{where}: `line` is only taken with `pointer`")
        _int(t["line"], f"{where}.line", 1)
    if "value_file" in t:
        if "pointer" not in t:
            raise CaseError(f"{where}: `value_file` is only taken with `pointer`")
        _expected_path(_str(t["value_file"], f"{where}.value_file"), f"{where}.value_file")
    if "includes" in t:
        if "pointer" not in t:
            raise CaseError(f"{where}: `includes` is only taken with `pointer`")
        _expect_type(t["includes"], (dict, list), f"{where}.includes", "a table or a list")
        _check_value_like(t["includes"], f"{where}.includes")
    if "value" in t:
        if "pointer" not in t and "post" not in t:
            raise CaseError(f"{where}: `value` is taken with `pointer` or `post`")
        _check_value_like(t["value"], f"{where}.value")
    if "dtype" in t:
        _str(t["dtype"], f"{where}.dtype")
    if "shape" in t:
        _expect_type(t["shape"], (list,), f"{where}.shape", "a list of integers")
        for i, n in enumerate(t["shape"]):
            _int(n, f"{where}.shape[{i}]", 0)
    if "post" in t:
        post = t["post"]
        _expect_type(post, (list,), f"{where}.post", "[i, j]")
        if len(post) != 2:
            raise CaseError(f"{where}.post: expected [i, j]")
        for i, n in enumerate(post):
            _int(n, f"{where}.post[{i}]", 0)
        if ("value" in t) == ("bits" in t):
            raise CaseError(f"{where}: `post` takes exactly one of `value` or `bits`")
        if "value" in t:
            _number(t["value"], f"{where}.value")
    if "plane" in t:
        if "post" not in t:
            raise CaseError(f"{where}: `plane` is only taken with `post`")
        _int(t["plane"], f"{where}.plane", 0)
    if "bits" in t:
        if "post" not in t:
            raise CaseError(f"{where}: `bits` is only taken with `post`")
        b = _str(t["bits"], f"{where}.bits")
        if not re.fullmatch(r"0x[0-9A-Fa-f]+", b):
            raise CaseError(f"{where}.bits: {b!r} is not 0x and hexadecimal digits")
    if "all" in t:
        _number(t["all"], f"{where}.all")
    if "tolerance" in t:
        tol = _number(t["tolerance"], f"{where}.tolerance")
        if tol < 0:
            raise CaseError(f"{where}.tolerance: below 0")
        if "all" not in t and not ("post" in t and "value" in t):
            raise CaseError(f"{where}: `tolerance` applies to `value` and `all` on array elements")
    checks = [k for k in t if k not in ("path", "stream", "ignore", "line", "plane", "tolerance")]
    if not checks:
        raise CaseError(f"{where}: names a file but checks nothing")


def _parse_diag(t: dict[str, Any], where: str) -> None:
    allowed = {
        "code", "severity", "subject", "file", "line", "column", "bytes", "argument", "position",
        "position_within", "count", "occurrences", "data", "data_has", "data_below", "related", "chain",
        "failed_on", "records", "from",
    }
    _keys(t, where, {"code"}, allowed - {"code"})
    _str(t["code"], f"{where}.code")
    for k in ("severity", "subject", "file"):
        if k in t:
            _str(t[k], f"{where}.{k}")
    if "severity" in t and t["severity"] not in ("error", "warning", "info"):
        raise CaseError(f"{where}.severity: {t['severity']!r} is not error, warning or info")
    for k in ("line", "column", "argument", "count", "occurrences"):
        if k in t:
            _int(t[k], f"{where}.{k}", 0)
    if "records" in t:
        _int(t["records"], f"{where}.records", 0)
    if "bytes" in t:
        b = t["bytes"]
        _expect_type(b, (list,), f"{where}.bytes", "2 integers")
        if len(b) != 2:
            raise CaseError(f"{where}.bytes: expected 2 integers")
        for i, n in enumerate(b):
            _int(n, f"{where}.bytes[{i}]", 0)
    if "position" in t:
        p = t["position"]
        _expect_type(p, (list,), f"{where}.position", "[x, y] or [x, y, z]")
        if len(p) not in (2, 3):
            raise CaseError(f"{where}.position: expected [x, y] or [x, y, z]")
        for i, n in enumerate(p):
            _number(n, f"{where}.position[{i}]")
    if "position_within" in t:
        b = t["position_within"]
        _expect_type(b, (list,), f"{where}.position_within", "[x0, y0, x1, y1]")
        if len(b) != 4:
            raise CaseError(f"{where}.position_within: expected [x0, y0, x1, y1]")
        for i, n in enumerate(b):
            _number(n, f"{where}.position_within[{i}]")
        if b[0] > b[2] or b[1] > b[3]:
            raise CaseError(f"{where}.position_within: x0 above x1 or y0 above y1")
    if "data" in t:
        _table(t["data"], f"{where}.data")
        _check_value_like(t["data"], f"{where}.data")
    if "data_has" in t:
        _expect_type(t["data_has"], (list,), f"{where}.data_has", "a list of member names")
        for i, n in enumerate(t["data_has"]):
            _str(n, f"{where}.data_has[{i}]")
    if "data_below" in t:
        for k, v in _table(t["data_below"], f"{where}.data_below").items():
            _str(v, f"{where}.data_below.{k}")
    if "related" in t:
        _expect_type(t["related"], (list,), f"{where}.related", "a list of tables")
        for i, r in enumerate(t["related"]):
            _keys(_table(r, f"{where}.related[{i}]"), f"{where}.related[{i}]", set(), {"file", "line"})
    if "chain" in t:
        _expect_type(t["chain"], (list,), f"{where}.chain", "a list of tables")
        for i, c in enumerate(t["chain"]):
            _keys(
                _table(c, f"{where}.chain[{i}]"),
                f"{where}.chain[{i}]",
                set(),
                {"through", "name", "file", "line", "owner"},
            )
    if "failed_on" in t:
        _expect_type(t["failed_on"], (bool,), f"{where}.failed_on", "a boolean")
    if "from" in t and t["from"] not in ("world", "run"):
        raise CaseError(f"{where}.from: {t['from']!r} is not world or run")


def _parse_nodes(t: dict[str, Any], where: str) -> None:
    allowed = {"ran", "ran_file", "ran_includes", "reused", "not_ran"}
    _keys(t, where, set(), allowed)
    if not t:
        raise CaseError(f"{where}: checks nothing")
    if "ran" in t and "ran_file" in t:
        raise CaseError(f"{where}: give `ran` or `ran_file`, not both")
    for k in ("ran", "ran_includes", "reused", "not_ran"):
        if k in t:
            _str_list(t[k], f"{where}.{k}")
    if "ran_file" in t:
        _expected_path(_str(t["ran_file"], f"{where}.ran_file"), f"{where}.ran_file")


def _parse_same(t: dict[str, Any], where: str, earlier_via: list[str]) -> None:
    _keys(t, where, {"action", "what"}, {"ignore", "differ"})
    a = _str(t["action"], f"{where}.action")
    if a not in earlier_via:
        raise CaseError(f"{where}.action: {a!r} is not an earlier via action of this case")
    what = _str(t["what"], f"{where}.what")
    if what not in SAME_WHAT_WORDS:
        _relative(what, f"{where}.what")
    if "ignore" in t:
        for i, p in enumerate(_str_list(t["ignore"], f"{where}.ignore")):
            if not p.startswith("/"):
                raise CaseError(f"{where}.ignore[{i}]: {p!r} is not a JSON Pointer to a member")
    if "differ" in t:
        _expect_type(t["differ"], (bool,), f"{where}.differ", "a boolean")


def _parse_named(t: dict[str, Any], where: str) -> None:
    fn = _str(t.get("fn"), f"{where}.fn") if "fn" in t else None
    if fn is None:
        raise CaseError(f"{where}: missing required key 'fn'")
    if fn not in NAMED_CHECKS:
        raise CaseError(f"{where}.fn: {fn!r} is not a named check of conformance.md 3.4")
    required, optional = NAMED_CHECKS[fn]
    args = {k: v for k, v in t.items() if k != "fn"}
    _keys(args, where, required, optional)
    for k in ("path", "before", "after", "build", "world", "names", "whole", "part", "input", "reference",
              "first", "second", "network", "field", "manifest"):
        if k in args:
            _relative(_str(args[k], f"{where}.{k}"), f"{where}.{k}")
    for k in ("fields", "kinds", "what", "fns", "entry_names", "causes"):
        if k in args:
            _str_list(args[k], f"{where}.{k}")
    for k in ("kinds",):
        for i, p in enumerate(args.get(k, [])):
            _relative(p, f"{where}.{k}[{i}]")
    if "prefix" in args:
        _str(args["prefix"], f"{where}.prefix")
    if "limit" in args:
        _number(args["limit"], f"{where}.limit")
    if "pointer" in args:
        ptr = _str(args["pointer"], f"{where}.pointer")
        if ptr and not ptr.startswith("/"):
            raise CaseError(f"{where}.pointer: {ptr!r} is not a JSON Pointer")
    if "count" in args:
        _int(args["count"], f"{where}.count", 0)
    if "bound" in args:
        _number(args["bound"], f"{where}.bound")
    if "distance" in args:
        _number(args["distance"], f"{where}.distance")
    if "window" in args:
        win = args["window"]
        _expect_type(win, (list,), f"{where}.window", "[i0, j0, i1, j1]")
        if len(win) != 4:
            raise CaseError(f"{where}.window: expected [i0, j0, i1, j1]")
        for i, n in enumerate(win):
            _int(n, f"{where}.window[{i}]", 0)
    if "node" in args:
        _str(args["node"], f"{where}.node")
    if "includes" in args:
        _table(args["includes"], f"{where}.includes")
        _check_value_like(args["includes"], f"{where}.includes")
    for k in ("from", "at_least"):
        if k in args:
            _int(args[k], f"{where}.{k}", 0)
    if "name" in args:
        _str(args["name"], f"{where}.name")
    for k in ("starts", "cause"):
        if k in args:
            _table(args[k], f"{where}.{k}")
            _check_value_like(args[k], f"{where}.{k}")
    if "starts" in args:
        for k, v in args["starts"].items():
            _str(v, f"{where}.starts.{k}")
    if "last" in args:
        _check_value_like({"last": args["last"]}, f"{where}")
    if fn in ("instance_at", "changes_inside"):
        at = args["at"]
        _expect_type(at, (list,), f"{where}.at", "[x, y]")
        if len(at) != 2:
            raise CaseError(f"{where}.at: expected [x, y]")
        for i, n in enumerate(at):
            _number(n, f"{where}.at[{i}]")
        if "radius" in args:
            _number(args["radius"], f"{where}.radius")
    if fn == "no_origin_inside":
        has_disc = "at" in args or "radius" in args
        if has_disc == ("points" in args):
            raise CaseError(f"{where}: give `at` and `radius`, or `points`")
        if has_disc:
            if "at" not in args or "radius" not in args:
                raise CaseError(f"{where}: `at` and `radius` go together")
            at = args["at"]
            _expect_type(at, (list,), f"{where}.at", "[x, y]")
            if len(at) != 2:
                raise CaseError(f"{where}.at: expected [x, y]")
            for i, n in enumerate(at):
                _number(n, f"{where}.at[{i}]")
            _number(args["radius"], f"{where}.radius")
        else:
            pts = args["points"]
            _expect_type(pts, (list,), f"{where}.points", "a list of [x, y]")
            if len(pts) < 3:
                raise CaseError(f"{where}.points: a polygon needs 3 points or more")
            for i, p in enumerate(pts):
                _expect_type(p, (list,), f"{where}.points[{i}]", "[x, y]")
                if len(p) != 2:
                    raise CaseError(f"{where}.points[{i}]: expected [x, y]")
                for n in p:
                    _number(n, f"{where}.points[{i}]")


def _parse_via(a: Action, earlier_via: list[str]) -> None:
    t = a.table
    where = f"action.{a.id}"
    allowed = {
        "via", "exit", "out", "env", "holding", "for_each", "diagnostics", "untouched",
        "pids_gone", "no_rerun", "diag", "file", "nodes", "same", "check",
    }
    _keys(t, where, {"via", "exit"}, allowed - {"via", "exit"})
    args = _str_list(t["via"], f"{where}.via")
    if not args:
        raise CaseError(f"{where}.via: no arguments")
    if "--diagnostics" in args:
        raise CaseError(f"{where}.via: the runner passes --diagnostics itself (3.5)")
    ex = t["exit"]
    if isinstance(ex, list):
        if not ex:
            raise CaseError(f"{where}.exit: an empty list allows nothing")
        for i, e in enumerate(ex):
            _int(e, f"{where}.exit[{i}]", 0, 255)
    else:
        _int(ex, f"{where}.exit", 0, 255)
    if args[0] in ("build", "lock") and "out" not in t:
        raise CaseError(f"{where}: `out` is required on `{args[0]}` (3.3)")
    if "out" in t:
        _relative(_str(t["out"], f"{where}.out"), f"{where}.out")
    if "env" in t:
        for k, v in _table(t["env"], f"{where}.env").items():
            _str(v, f"{where}.env.{k}")
    for k in ("holding", "pids_gone"):
        if k in t:
            _relative(_str(t[k], f"{where}.{k}"), f"{where}.{k}")
    if "for_each" in t:
        _relative(_str(t["for_each"], f"{where}.for_each"), f"{where}.for_each")
    if "diagnostics" in t and _str(t["diagnostics"], f"{where}.diagnostics") not in DIAG_MODES:
        raise CaseError(f"{where}.diagnostics: {t['diagnostics']!r} is not exact, includes or any")
    if "untouched" in t:
        for i, p in enumerate(_str_list(t["untouched"], f"{where}.untouched")):
            _relative(p, f"{where}.untouched[{i}]")
    if "no_rerun" in t:
        reason = _str(t["no_rerun"], f"{where}.no_rerun")
        if not reason.strip():
            raise CaseError(f"{where}.no_rerun: give the reason")
    for cid, d in _table(t.get("diag", {}), f"{where}.diag").items():
        _parse_diag(_table(d, f"{where}.diag.{cid}"), f"{where}.diag.{cid}")
    if t.get("diagnostics") == "any" and t.get("diag"):
        raise CaseError(f"{where}: mode `any` compares no expectation; remove the diag tables")
    for cid, f in _table(t.get("file", {}), f"{where}.file").items():
        _parse_file_check(_table(f, f"{where}.file.{cid}"), f"{where}.file.{cid}")
    if "nodes" in t:
        if args[0] not in ("build", "lock"):
            raise CaseError(f"{where}.nodes: only a build or lock action writes a run report")
        _parse_nodes(_table(t["nodes"], f"{where}.nodes"), f"{where}.nodes")
    for cid, s in _table(t.get("same", {}), f"{where}.same").items():
        _parse_same(_table(s, f"{where}.same.{cid}"), f"{where}.same.{cid}", earlier_via)
    for cid, c in _table(t.get("check", {}), f"{where}.check").items():
        _parse_named(_table(c, f"{where}.check.{cid}"), f"{where}.check.{cid}")


def _parse_edit(a: Action) -> None:
    t = a.table
    where = f"action.{a.id}"
    _relative(_str(t[a.kind], f"{where}.{a.kind}"), f"{where}.{a.kind}")
    if a.kind == "replace":
        _keys(t, where, {"replace", "find", "with"}, set())
        if not _str(t["find"], f"{where}.find"):
            raise CaseError(f"{where}.find: empty")
        _str(t["with"], f"{where}.with")
    elif a.kind == "write":
        _keys(t, where, {"write"}, {"text", "from"})
        if ("text" in t) == ("from" in t):
            raise CaseError(f"{where}: give exactly one of `text` or `from`")
        if "text" in t:
            _str(t["text"], f"{where}.text")
        else:
            src = _relative(_str(t["from"], f"{where}.from"), f"{where}.from")
            if not src.startswith("later/"):
                raise CaseError(f"{where}.from: {src!r} must be a path under later/")
    elif a.kind == "delete":
        _keys(t, where, {"delete"}, set())
    elif a.kind == "flip":
        _keys(t, where, {"flip", "at"}, set())
        _int(t["at"], f"{where}.at", 0)
    elif a.kind == "flip_object":
        _keys(t, where, {"flip_object", "node", "store", "at"}, set())
        _str(t["node"], f"{where}.node")
        _relative(_str(t["store"], f"{where}.store"), f"{where}.store")
        _int(t["at"], f"{where}.at", 0)
    elif a.kind == "rerecord":
        _keys(t, where, {"rerecord", "manifest"}, set())
        _relative(_str(t["manifest"], f"{where}.manifest"), f"{where}.manifest")
    elif a.kind == "truncate":
        _keys(t, where, {"truncate", "bytes"}, set())
        _int(t["bytes"], f"{where}.bytes", 0)
    elif a.kind == "derive":
        _keys(t, where, {"derive", "from", "as"}, {"input", "params"})
        _relative(_str(t["from"], f"{where}.from"), f"{where}.from")
        if t["as"] not in DERIVE_AS:
            raise CaseError(f"{where}.as: {t['as']!r} is not one of {', '.join(DERIVE_AS)}")
        if "input" in t:
            _relative(_str(t["input"], f"{where}.input"), f"{where}.input")
        needs = {"segments": set(), "cells": {"input", "params"}, "interpolate": {"input"},
                 "project": {"input"}, "params": {"params"}}[t["as"]]
        given = {k for k in ("input", "params") if k in t}
        if given != needs:
            want = " and ".join(f"`{k}`" for k in sorted(needs)) or "neither `input` nor `params`"
            raise CaseError(f"{where}: `as = \"{t['as']}\"` takes {want}")
        if "params" in t:
            _table(t["params"], f"{where}.params")


def check_id(value: str, where: str) -> None:
    if not ID_RE.match(value) or len(value.encode("utf-8")) > MAX_ID_BYTES:
        raise CaseError(f"{where}: {value!r} is not an id ([a-z][a-z0-9_]*, at most 48 bytes)")


def parse_case(folder: Path, chapter: str) -> Case:
    """Read and check one case folder. Raises CaseError."""
    check_id(folder.name, "case folder")
    manifest = folder / "case.toml"
    if not manifest.is_file():
        raise CaseError(f"{folder.name}: no case.toml")
    try:
        doc = tomllib.loads(manifest.read_bytes().decode("utf-8"))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as e:
        raise CaseError(f"case.toml does not parse: {e}") from e

    top = {"format", "summary", "milestone", "actions", "pending", "one_system", "copy", "action"}
    _keys(doc, "case.toml", {"format", "summary", "milestone", "actions"}, top - {
        "format", "summary", "milestone", "actions"})
    if _str(doc["format"], "format") != FORMAT:
        raise CaseError(f"format: {doc['format']!r} is not {FORMAT!r}")
    summary = _str(doc["summary"], "summary")
    if not summary.strip() or "\n" in summary:
        raise CaseError("summary: one non-empty line")
    milestone = _int(doc["milestone"], "milestone", 0, 7)
    pending = _str(doc["pending"], "pending") if "pending" in doc else None
    if pending is not None and not pending.strip():
        raise CaseError("pending: give the reason")
    one_system = _str(doc["one_system"], "one_system") if "one_system" in doc else None
    if one_system is not None and not one_system.strip():
        raise CaseError("one_system: give the reason")

    copies: list[Copy] = []
    for cid in sorted(_table(doc.get("copy", {}), "copy")):
        where = f"copy.{cid}"
        t = _table(doc["copy"][cid], where)
        _keys(t, where, {"from"}, {"to"})
        src = _relative(_str(t["from"], f"{where}.from"), f"{where}.from")
        if PurePosixPath(src).parts[0] == "spike":
            raise CaseError(f"{where}.from: a case never copies from spike/ (3.2)")
        to = _str(t.get("to", src), f"{where}.to")
        if to != ".":
            _relative(to, f"{where}.to")
        copies.append(Copy(cid, src, to))

    order = _str_list(doc["actions"], "actions")
    if not order:
        raise CaseError("actions: a case runs at least one action")
    tables = _table(doc.get("action", {}), "action")
    seen: set[str] = set()
    for aid in order:
        if aid in seen:
            raise CaseError(f"actions: {aid!r} is listed twice")
        seen.add(aid)
        if aid not in tables:
            raise CaseError(f"actions: {aid!r} names no [action.{aid}] table")
    for aid in tables:
        if aid not in seen:
            raise CaseError(f"[action.{aid}] is not listed in actions")

    actions: list[Action] = []
    earlier_via: list[str] = []
    for aid in order:
        check_id(aid, f"action id {aid!r}")
        t = _table(tables[aid], f"action.{aid}")
        kinds = [k for k in ACTION_KINDS if k in t]
        if len(kinds) != 1:
            raise CaseError(
                f"action.{aid}: holds exactly one of {', '.join(ACTION_KINDS)}; found {kinds or 'none'}"
            )
        a = Action(aid, kinds[0], t)
        if a.kind == "via":
            _parse_via(a, earlier_via)
            earlier_via.append(aid)
        else:
            _parse_edit(a)
        actions.append(a)

    return Case(
        chapter=chapter,
        id=folder.name,
        folder=folder,
        summary=summary,
        milestone=milestone,
        actions=actions,
        copies=copies,
        pending=pending,
        one_system=one_system,
    )


@dataclass
class Discovered:
    chapter: str
    folder: Path
    case: Case | None
    error: str | None


def discover(cases_root: Path) -> list[Discovered]:
    """Every folder spec/cases/<chapter>/<case>/, in bytewise order of chapter, then case id."""
    found: list[Discovered] = []
    if not cases_root.is_dir():
        return found
    chapters = sorted((p for p in cases_root.iterdir() if p.is_dir()), key=lambda p: p.name.encode())
    for ch in chapters:
        for folder in sorted((p for p in ch.iterdir() if p.is_dir()), key=lambda p: p.name.encode()):
            try:
                found.append(Discovered(ch.name, folder, parse_case(folder, ch.name), None))
            except CaseError as e:
                found.append(Discovered(ch.name, folder, None, str(e)))
    return found
