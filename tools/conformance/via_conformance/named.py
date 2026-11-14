"""Named checks (conformance.md 3.4, `[action.<a>.check.<id>]`).

Each is a property that is not a value. The table in conformance.md 3.4 is the one list; a name not
in it is a case error (model.NAMED_CHECKS). Every check reads build files with numpy and the
standard library only, and returns a list of failures: [] when it holds.
"""

from __future__ import annotations

import math
import tomllib
from fractions import Fraction
from pathlib import Path
from typing import Any, Callable

import numpy as np

from . import blake3, canon, files, kernel

Failure = tuple[str, str, str]


class NamedCheckError(Exception):
    """A named check could not read its inputs; reported as a failure naming the input."""


def _read_json(path: Path) -> Any:
    try:
        return canon.loads(path.read_bytes())
    except (OSError, ValueError) as e:
        raise NamedCheckError(f"cannot read {path.name}: {e}") from e


def _arrays(path: Path) -> list[tuple[str, np.ndarray]]:
    if path.is_dir():
        found = sorted(path.rglob("*.npy"), key=lambda p: str(p).encode())
        if not found:
            raise NamedCheckError(f"{path.name}/ holds no array file")
        return [(p.relative_to(path).as_posix(), files.load_array(p)) for p in found]
    return [(path.name, files.load_array(path))]


def _post_sums(arr: np.ndarray) -> np.ndarray:
    """Per-post sum of a weights file: (planes, rows, columns) summed over planes."""
    if arr.ndim == 3:
        return np.asarray(arr, dtype=np.int64).sum(axis=0)
    if arr.ndim == 2:
        return np.asarray(arr, dtype=np.int64)
    raise NamedCheckError(f"a weights file has 2 or 3 dimensions, not {arr.ndim}")


def weights_sum_at_most(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    fails: list[Failure] = []
    for name, arr in _arrays(work / args["path"]):
        sums = _post_sums(arr)
        bad = np.argwhere(sums > args["limit"])
        if bad.size:
            j, i = bad[0]
            fails.append((f"check.{cid}", f"every post of {name} sums to at most {args['limit']}",
                          f"post ({i}, {j}) sums to {int(sums[j, i])}"))
    return fails


def weights_sum_255_or_0(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    fails: list[Failure] = []
    for name, arr in _arrays(work / args["path"]):
        sums = _post_sums(arr)
        bad = np.argwhere((sums != 255) & (sums != 0))
        if bad.size:
            j, i = bad[0]
            fails.append((f"check.{cid}", f"every post of {name} sums to 255 or 0",
                          f"post ({i}, {j}) sums to {int(sums[j, i])}"))
    return fails


def no_absolute_path(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    problems: list[str] = ctx["u5"]()
    return [(f"check.{cid}", "no absolute path, user or host name", p) for p in problems]


# --- changed_within_windows -------------------------------------------------------------------


def _mm(x: float | int) -> int:
    """Metres as written in a build file to integer millimetres; build files hold lattice values,
    so the product is integral (numbers.md 3.4)."""
    return round(Fraction(x) * 1000)


def _plan_shape(entry: dict[str, Any]) -> tuple:
    """A feature's or footprint's plan shape, x and y only, never z (fields.md change windows)."""
    if "at" in entry and entry["at"] is not None:
        return ("at", (_mm(entry["at"][0]), _mm(entry["at"][1])))
    if "points" in entry and entry["points"] is not None:
        return ("points", tuple((_mm(p[0]), _mm(p[1])) for p in entry["points"]))
    if "pieces" in entry and entry["pieces"] is not None:
        out = []
        for piece in entry["pieces"]:
            rings = [piece.get("outer", [])] + list(piece.get("holes", []))
            out.append(tuple(tuple((_mm(p[0]), _mm(p[1])) for p in ring) for ring in rings))
        return ("pieces", tuple(out))
    return ("none", ())


def _bbox(shape: tuple) -> tuple[int, int, int, int] | None:
    kind, data = shape
    pts: list[tuple[int, int]] = []
    if kind == "at":
        pts = [data]
    elif kind == "points":
        pts = list(data)
    elif kind == "pieces":
        pts = [p for piece in data for ring in piece for p in ring]
    if not pts:
        return None
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def _kind_radii(kinds_files: list[Path]) -> dict[str, float | None]:
    """Each kind's largest effect radius, each effect's own else the kind's; None when an effect
    has neither (it is evaluated over the whole extent, fields.md 5.3)."""
    radii: dict[str, float | None] = {}
    for path in kinds_files:
        try:
            doc = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError, UnicodeDecodeError) as e:
            raise NamedCheckError(f"cannot read kinds file {path.name}: {e}") from e
        for name, kind in doc.get("kind", {}).items():
            if not isinstance(kind, dict):
                continue
            kind_r = kind.get("radius")
            effects = kind.get("effect", {})
            best: float | None = 0.0
            for eff in effects.values() if isinstance(effects, dict) else []:
                r = eff.get("radius", kind_r) if isinstance(eff, dict) else kind_r
                if r is None:
                    best = None
                    break
                best = max(best, float(r))
            radii[name] = best if effects else 0.0
    return radii


def _frame(manifest: dict[str, Any]) -> tuple[Fraction, Fraction, Fraction, int, int]:
    fr = manifest.get("frame")
    if not isinstance(fr, dict):
        raise NamedCheckError("manifest.json has no frame")
    x0, y0 = Fraction(fr["extent"][0]), Fraction(fr["extent"][1])
    r = Fraction(fr["resolution"])
    nx, ny = fr["posts"]
    return x0, y0, r, int(nx), int(ny)


def _window(box_mm: tuple[int, int, int, int] | None, radius: float | None,
            frame: tuple[Fraction, Fraction, Fraction, int, int]) -> tuple[int, int, int, int]:
    """fields.md 5.3: i0 = max(0, floor((xmin - radius - x0) / r)),
    i1 = min(nx, ceil((xmax + radius - x0) / r) + 1), j alike."""
    x0, y0, r, nx, ny = frame
    if radius is None or box_mm is None:
        return 0, 0, nx, ny
    rad = Fraction(radius)
    xmin, ymin, xmax, ymax = (Fraction(v, 1000) for v in box_mm)
    i0 = max(0, math.floor((xmin - rad - x0) / r))
    i1 = min(nx, math.ceil((xmax + rad - x0) / r) + 1)
    j0 = max(0, math.floor((ymin - rad - y0) / r))
    j1 = min(ny, math.ceil((ymax + rad - y0) / r) + 1)
    return i0, j0, i1, j1


def _things(features: dict[str, Any]) -> dict[tuple[str, str], dict[str, Any]]:
    out: dict[tuple[str, str], dict[str, Any]] = {}
    for f in features.get("features", []):
        out[("feature", f["name"])] = f
    for f in features.get("footprints", []):
        out[("footprint", f["instance"])] = f
    return out


def changed_within_windows(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    before, after = work / args["before"], work / args["after"]
    radii = _kind_radii([work / k for k in args["kinds"]])
    m_before = _read_json(before / "manifest.json")
    m_after = _read_json(after / "manifest.json")
    frame = _frame(m_after)
    if _frame(m_before) != frame:
        return [(f"check.{cid}", "both builds on one frame", "the frames differ")]
    things_b = _things(_read_json(before / "features.json"))
    things_a = _things(_read_json(after / "features.json"))
    x0, y0, r, nx, ny = frame
    allowed = np.zeros((ny, nx), dtype=bool)
    moved: list[str] = []
    for key in sorted(set(things_b) | set(things_a)):
        b, a = things_b.get(key), things_a.get(key)
        sb = _plan_shape(b) if b else None
        sa = _plan_shape(a) if a else None
        if sb == sa:
            continue
        moved.append(f"{key[0]} {key[1]}")
        kind = (a or b).get("kind")
        radius = radii.get(kind, 0.0) if kind is not None else 0.0
        for shape in (sb, sa):
            if shape is None:
                continue
            i0, j0, i1, j1 = _window(_bbox(shape), radius, frame)
            allowed[j0:j1, i0:i1] = True
    fails: list[Failure] = []
    for name in args["fields"]:
        fb = _field_file(m_before, before, name)
        fa = _field_file(m_after, after, name)
        ab, aa = files.load_array(fb), files.load_array(fa)
        if ab.shape != aa.shape or ab.shape != (ny, nx):
            fails.append((f"check.{cid}", f"{name}: shape ({ny}, {nx}) in both", f"{ab.shape} and {aa.shape}"))
            continue
        differ = np.asarray(ab).view(np.uint8).reshape(ny, nx, -1) != np.asarray(aa).view(np.uint8).reshape(ny, nx, -1)
        differ = differ.any(axis=2)
        outside = np.argwhere(differ & ~allowed)
        if outside.size:
            j, i = outside[0]
            fails.append((f"check.{cid}",
                          f"{name}: every differing post inside the windows of {', '.join(moved) or 'nothing moved'}",
                          f"{len(outside)} posts outside, first ({i}, {j})"))
    return fails


def _field_file(manifest: dict[str, Any], build: Path, name: str) -> Path:
    """The field's array file, by the manifest's entry when it names one (readers never build a
    file name from a name, build.md 3.4), else `fields/<name>.npy`."""
    fields = manifest.get("fields")
    entry = fields.get(name) if isinstance(fields, dict) else None
    if isinstance(entry, dict) and isinstance(entry.get("file"), str):
        return build / entry["file"]
    return build / "fields" / f"{name}.npy"


# --- no_origin_inside --------------------------------------------------------------------------


def _inside_polygon(px: int, py: int, poly: list[tuple[int, int]]) -> bool:
    """Even-odd rule; a point on the boundary counts as inside, the stricter reading."""
    inside = False
    n = len(poly)
    for k in range(n):
        (x1, y1), (x2, y2) = poly[k], poly[(k + 1) % n]
        cross = (x2 - x1) * (py - y1) - (y2 - y1) * (px - x1)
        if cross == 0 and min(x1, x2) <= px <= max(x1, x2) and min(y1, y2) <= py <= max(y1, y2):
            return True
        if (y1 > py) != (y2 > py):
            # x of the edge at py, compared without division.
            lhs = (px - x1) * (y2 - y1)
            rhs = (x2 - x1) * (py - y1)
            if (lhs < rhs) == (y2 > y1):
                inside = not inside
    return inside


def no_origin_inside(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    build = work / args["build"]
    xs = files.load_array(build / "instances" / "x.npy")
    ys = files.load_array(build / "instances" / "y.npy")
    names = (build / "instances" / "names.txt").read_text(encoding="utf-8").splitlines()
    if not (len(xs) == len(ys) == len(names)):
        return [(f"check.{cid}", "x, y and names.txt of one length", f"{len(xs)}, {len(ys)}, {len(names)}")]
    prefix = args.get("prefix", "")
    hits: list[str] = []
    if "at" in args:
        ax, ay = _mm(args["at"][0]), _mm(args["at"][1])
        rad = Fraction(args["radius"]) * 1000
        for name, x, y in zip(names, xs, ys):
            if name.startswith(prefix):
                dx, dy = int(x) - ax, int(y) - ay
                if dx * dx + dy * dy <= rad * rad:
                    hits.append(name)
        where = f"within {args['radius']} m of {args['at']}"
    else:
        poly = [(_mm(p[0]), _mm(p[1])) for p in args["points"]]
        for name, x, y in zip(names, xs, ys):
            if name.startswith(prefix) and _inside_polygon(int(x), int(y), poly):
                hits.append(name)
        where = "inside the polygon"
    if hits:
        return [(f"check.{cid}", f"no origin {where}" + (f" among {prefix!r}*" if prefix else ""),
                 f"{len(hits)}: {', '.join(hits[:5])}")]
    return []


# --- changes_by_prefix -------------------------------------------------------------------------


def changes_by_prefix(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    report = _read_json(work / args["path"])
    entries = report.get("entries") if isinstance(report, dict) else None
    if not isinstance(entries, list):
        return [(f"check.{cid}", "a change report with entries", "no `entries` list")]
    prefix, what = args["prefix"], args["what"]
    mine = [e for e in entries if isinstance(e, dict) and str(e.get("name", "")).startswith(prefix)]
    fails: list[Failure] = []
    if not mine:
        fails.append((f"check.{cid}", f"an entry named {prefix}*", "none"))
    wrong = [e for e in mine if e.get("what") != what]
    if wrong:
        fails.append((f"check.{cid}", f"every {prefix}* entry is {canon.show(what)}",
                      f"{wrong[0].get('name')} is {canon.show(wrong[0].get('what'))}"))
    if "names" in args:
        named = {e.get("name") for e in mine}
        listed = [
            n for n in (work / args["names"]).read_text(encoding="utf-8").splitlines() if n.startswith(prefix)
        ]
        missing = [n for n in listed if n not in named]
        if missing:
            fails.append((f"check.{cid}", f"every {prefix}* line of {args['names']} is an entry",
                          f"{len(missing)} missing, first {missing[0]}"))
    return fails


# --- vectors_match ----------------------------------------------------------------------------


def vectors_match(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """build.md 14: `via vectors FILE` prints one `{"got","id"}` object per line, in the file's
    order; the runner compares each `got` with the vector's `expected`. With `fns`, only the
    vectors whose `fn` is listed are compared, and each of them must be printed."""
    try:
        doc = tomllib.loads((work / args["path"]).read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError, UnicodeDecodeError) as e:
        raise NamedCheckError(f"cannot read vector file {args['path']}: {e}") from e
    vectors = doc.get("vector", {})
    fns = set(args.get("fns", []))
    want = {vid: v for vid, v in vectors.items()
            if isinstance(v, dict) and (not fns or v.get("fn") in fns)}
    got: dict[str, Any] = {}
    for n, line in enumerate(ctx.get("stdout", b"").decode("utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            obj = canon.loads(line.encode("utf-8"))
        except ValueError as e:
            return [(f"check.{cid}", "one JSON object per line", f"line {n}: {e}")]
        if isinstance(obj, dict) and "id" in obj:
            got[obj["id"]] = obj.get("got")
    fails: list[Failure] = []
    for vid, v in sorted(want.items()):
        if vid not in got:
            fails.append((f"check.{cid}", f"vector {vid} printed", "absent"))
        elif not _vector_equal(got[vid], v.get("expected")):
            fails.append((f"check.{cid}", f"vector {vid}: {canon.show(v.get('expected'))}", canon.show(got[vid])))
    return fails


def _vector_equal(a: Any, b: Any) -> bool:
    """Exact equality; hexadecimal bit patterns compare ignoring letter case, numbers by value."""
    if isinstance(a, str) and isinstance(b, str):
        return a.upper() == b.upper() if a.lower().startswith("0x") else a == b
    if isinstance(a, bool) or isinstance(b, bool):
        return a is b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return Fraction(a) == Fraction(b)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(_vector_equal(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(_vector_equal(a[k], b[k]) for k in a)
    return False


# --- json_count -------------------------------------------------------------------------------


def pack_root_resolves(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """`build.md` 4: the manifest's `locations.pack_root`, a path from the build directory, names
    an existing folder that holds every file `assets.files` lists."""
    build = work / args["build"]
    m = _read_json(build / "manifest.json")
    root = (m.get("locations") or {}).get("pack_root")
    if not isinstance(root, str):
        return [(f"check.{cid}", f"{args['build']}: pack_root is a path", repr(root))]
    folder = build / root
    if not folder.is_dir():
        return [(f"check.{cid}", f"{args['build']}: pack_root {root} is a folder", "absent")]
    missing = []
    for a in (m.get("assets") or {}).values():
        files = a.get("files") if isinstance(a, dict) else None
        for f in (files.values() if isinstance(files, dict) else files or []):
            if isinstance(f, str) and not (folder / f).is_file():
                missing.append(f)
    if missing:
        return [(f"check.{cid}", f"{args['build']}: asset files under pack_root", ", ".join(sorted(missing)))]
    return []


def json_count(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """The list or object a JSON Pointer names in a JSON file has exactly `count` members."""
    doc = _read_json(work / args["path"])
    node: Any = doc
    for part in [p.replace("~1", "/").replace("~0", "~") for p in args["pointer"].split("/")[1:]]:
        if isinstance(node, list) and part.isdigit() and int(part) < len(node):
            node = node[int(part)]
        elif isinstance(node, dict) and part in node:
            node = node[part]
        else:
            return [(f"check.{cid}", f"{args['pointer']} in {args['path']}", "absent")]
    if not isinstance(node, (list, dict)):
        return [(f"check.{cid}", f"{args['pointer']} is a list or object", type(node).__name__)]
    if len(node) != args["count"]:
        return [(f"check.{cid}", f"{args['pointer']} holds {args['count']}", str(len(node)))]
    return []


# --- names_count, names_kept (geometry.md 8, identity.md 23 to 26) ---------------------------


def _names_of(path: Path) -> dict[str, list[tuple[int, int]]]:
    """Names in a `features.json` (`build.md` 9.1), each with its outer-ring vertices in
    millimetres, or the lines of a text file such as `instances/names.txt`: with its origin when
    `x.npy` and `y.npy` of the same length lie beside it, else with no vertices."""
    if path.suffix == ".json":
        doc = _read_json(path)
        feats = doc.get("features") if isinstance(doc, dict) else None
        if not isinstance(feats, list):
            raise NamedCheckError(f"{path.name}: no `features` list")
        out: dict[str, list[tuple[int, int]]] = {}
        for f in feats:
            if not isinstance(f, dict) or not isinstance(f.get("name"), str):
                continue
            pts: list[tuple[int, int]] = []
            for piece in f.get("pieces") or []:
                pts.extend((_mm(p[0]), _mm(p[1])) for p in (piece.get("outer") or []))
            for p in f.get("points") or []:
                pts.append((_mm(p[0]), _mm(p[1])))
            if isinstance(f.get("at"), list):
                pts.append((_mm(f["at"][0]), _mm(f["at"][1])))
            out[f["name"]] = pts
        return out
    names = [line for line in path.read_text(encoding="utf-8").splitlines() if line]
    # An instance names file (build.md 9.2) has its origins beside it, in millimetres: each name
    # carries its origin, so `except_within` applies to instances as to features.
    xs, ys = path.parent / "x.npy", path.parent / "y.npy"
    if path.name == "names.txt" and xs.is_file() and ys.is_file():
        x, y = np.load(xs), np.load(ys)
        if len(x) == len(names) == len(y):
            return {n: [(int(x[i]), int(y[i]))] for i, n in enumerate(names)}
    return {n: [] for n in names}


def names_count(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """Exactly `count` names in `path` start with `prefix`."""
    names = [n for n in _names_of(work / args["path"]) if n.startswith(args["prefix"])]
    if len(names) != args["count"]:
        return [(f"check.{cid}", f"{args['count']} names starting {args['prefix']!r} in {args['path']}",
                 str(len(names)))]
    return []


def names_kept(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """The names starting with `prefix` are the same set in `first` and `second`, and not empty.
    With `except_within` (`[x0, y0, x1, y1]` metres, edges excluded), a name whose feature has a
    vertex strictly inside the box in either file is left out of both sides first."""
    a, b = _names_of(work / args["first"]), _names_of(work / args["second"])
    box = args.get("except_within")

    def near(pts: list[tuple[int, int]]) -> bool:
        if box is None:
            return False
        x0, y0, x1, y1 = (_mm(v) for v in box)
        return any(x0 < x < x1 and y0 < y < y1 for x, y in pts)

    skip = {n for n, p in list(a.items()) + list(b.items()) if near(p)}
    sa = {n for n in a if n.startswith(args["prefix"]) and n not in skip}
    sb = {n for n in b if n.startswith(args["prefix"]) and n not in skip}
    if not sa:
        return [(f"check.{cid}", f"names starting {args['prefix']!r} in {args['first']}", "none")]
    if sa != sb:
        gone, new = sorted(sa - sb), sorted(sb - sa)
        return [(f"check.{cid}", f"the same names starting {args['prefix']!r}",
                 f"{len(gone)} gone ({', '.join(gone[:3])}), {len(new)} new ({', '.join(new[:3])})")]
    return []


# --- run reports (graph.md 6.3) ---------------------------------------------------------------


def _members_include(got: Any, want: Any) -> bool:
    if isinstance(want, dict):
        return isinstance(got, dict) and all(k in got and _members_include(got[k], v) for k, v in want.items())
    if isinstance(want, bool) or isinstance(got, bool):
        return got is want
    if isinstance(want, (int, float)) and isinstance(got, (int, float)):
        return Fraction(got) == Fraction(want)
    return got == want


def run_node(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """The run report's node object with id `node` exists and holds the members of `includes`."""
    report = _read_json(work / args["path"])
    nodes = report.get("nodes") if isinstance(report, dict) else None
    if not isinstance(nodes, list):
        return [(f"check.{cid}", "a run report with nodes", "no `nodes` list")]
    found = [n for n in nodes if isinstance(n, dict) and n.get("id") == args["node"]]
    if not found:
        return [(f"check.{cid}", f"node {args['node']}", "absent")]
    if not _members_include(found[0], args["includes"]):
        return [(f"check.{cid}", f"node {args['node']} includes {canon.show(args['includes'])}", canon.show(found[0]))]
    return []


RUN_MEMBERS = {"build", "diagnostics", "engine", "format", "nodes", "steps", "system", "threads", "totals"}


def run_report_valid(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """graph.md 6.3: canonical JSON, format via-run-0, every member; nodes once each in id order;
    totals equal the statuses; every ran node has a cause, every other node's cause is null."""
    path = work / args["path"]
    try:
        raw = path.read_bytes()
    except OSError as e:
        raise NamedCheckError(f"cannot read {args['path']}: {e}") from e
    report = _read_json(path)
    fails: list[Failure] = []
    if not canon.is_canonical(raw):
        fails.append((f"check.{cid}", "the run report is canonical JSON", "it is not"))
    if not isinstance(report, dict):
        return fails + [(f"check.{cid}", "an object", type(report).__name__)]
    if report.get("format") != "via-run-0":
        fails.append((f"check.{cid}", "format via-run-0", canon.show(report.get("format"))))
    missing = sorted(RUN_MEMBERS - set(report))
    if missing:
        fails.append((f"check.{cid}", "every member of graph.md 6.3", f"missing {', '.join(missing)}"))
    nodes = report.get("nodes") if isinstance(report.get("nodes"), list) else []
    ids = [n.get("id") for n in nodes if isinstance(n, dict)]
    if ids != sorted(ids, key=lambda s: str(s).encode()) or len(set(ids)) != len(ids):
        fails.append((f"check.{cid}", "nodes once each, in id order", "they are not"))
    counts = {"ran": 0, "recomputed": 0, "reused": 0, "skipped": 0}
    for n in nodes:
        st = n.get("status")
        if st in counts:
            counts[st] += 1
        if st == "ran" and not n.get("cause"):
            fails.append((f"check.{cid}", f"ran node {n.get('id')} has a cause", "null"))
        if st != "ran" and n.get("cause") is not None:
            fails.append((f"check.{cid}", f"{st} node {n.get('id')} has cause null", canon.show(n.get("cause"))))
    totals = report.get("totals") if isinstance(report.get("totals"), dict) else {}
    for k, v in counts.items():
        if totals.get(k) != v:
            fails.append((f"check.{cid}", f"totals.{k} = {v}", canon.show(totals.get(k))))
    if totals.get("nodes") != len(nodes):
        fails.append((f"check.{cid}", f"totals.nodes = {len(nodes)}", canon.show(totals.get("nodes"))))
    return fails


# --- array_window -----------------------------------------------------------------------------


def array_window(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """`part` (a 2-D array file) equals, element by element and bit for bit, the window
    `[i0, j0, i1, j1]` (i1, j1 excluded) of `whole`: numbers.md 6's window rule, `build.md` 14's
    `via eval --window`."""
    whole = files.load_array(work / args["whole"])
    part = files.load_array(work / args["part"])
    i0, j0, i1, j1 = args["window"]
    if whole.ndim != 2 or part.ndim != 2:
        return [(f"check.{cid}", "two 2-D array files", f"{whole.ndim} and {part.ndim} dimensions")]
    cut = whole[j0:j1, i0:i1]
    if cut.shape != part.shape:
        return [(f"check.{cid}", f"shape {cut.shape}", f"{part.shape}")]
    a = np.ascontiguousarray(cut).view(np.uint8).reshape(cut.shape[0], cut.shape[1], -1)
    b = np.ascontiguousarray(part).view(np.uint8).reshape(part.shape[0], part.shape[1], -1)
    if cut.dtype != part.dtype or a.shape != b.shape:
        return [(f"check.{cid}", f"dtype {cut.dtype}", f"{part.dtype}")]
    bad = np.argwhere((a != b).any(axis=2))
    if bad.size:
        j, i = bad[0]
        return [(f"check.{cid}", f"window {args['window']} of {args['whole']} bit for bit",
                 f"{len(bad)} posts differ, first ({i0 + i}, {j0 + j})")]
    return []


# --- json_list --------------------------------------------------------------------------------


def _starts(got: Any, prefixes: dict[str, str]) -> bool:
    return isinstance(got, dict) and all(isinstance(got.get(k), str) and got[k].startswith(p)
                                         for k, p in prefixes.items())


def json_list(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """The list at `pointer`: at least `at_least` (default 1) elements from index `from` (default
    0); each of those includes `includes`, has each member of `starts` beginning with that
    prefix and each member named in `nulls` present and null; its last element equals `last`."""
    doc = _read_json(work / args["path"])
    try:
        node = canon.resolve(doc, args["pointer"])
    except canon.PointerError as e:
        return [(f"check.{cid}", f"{args['pointer']} resolves", str(e))]
    if not isinstance(node, list):
        return [(f"check.{cid}", f"{args['pointer']} is a list", type(node).__name__)]
    start = args.get("from", 0)
    tail = node[start:]
    fails: list[Failure] = []
    at_least = args.get("at_least", 1)
    if len(tail) < at_least:
        fails.append((f"check.{cid}", f"at least {at_least} elements from index {start}", str(len(tail))))
    for i, e in enumerate(tail, start):
        if "includes" in args and not _members_include(e, args["includes"]):
            fails.append((f"check.{cid}", f"element {i} includes {canon.show(args['includes'])}", canon.show(e)))
            break
        if "starts" in args and not _starts(e, args["starts"]):
            fails.append((f"check.{cid}", f"element {i} starts {canon.show(args['starts'])}", canon.show(e)))
            break
        if "nulls" in args and not (isinstance(e, dict) and all(k in e and e[k] is None for k in args["nulls"])):
            fails.append((f"check.{cid}", f"element {i} has {', '.join(args['nulls'])} null", canon.show(e)))
            break
    if "last" in args and (not node or not canon.values_equal(args["last"], node[-1])):
        fails.append((f"check.{cid}", f"last element {canon.show(args['last'])}",
                      canon.show(node[-1]) if node else "an empty list"))
    return fails


# --- instance_at ------------------------------------------------------------------------------


def instance_at(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """The build holds exactly one instance named `name`, its origin in plan exactly at `at`
    (integer millimetres, build.md 8)."""
    build = work / args["build"]
    xs = files.load_array(build / "instances" / "x.npy")
    ys = files.load_array(build / "instances" / "y.npy")
    names = (build / "instances" / "names.txt").read_text(encoding="utf-8").splitlines()
    if not (len(xs) == len(ys) == len(names)):
        return [(f"check.{cid}", "x, y and names.txt of one length", f"{len(xs)}, {len(ys)}, {len(names)}")]
    rows = [k for k, n in enumerate(names) if n == args["name"]]
    if len(rows) != 1:
        return [(f"check.{cid}", f"one instance named {args['name']}", str(len(rows)))]
    want = (_mm(args["at"][0]), _mm(args["at"][1]))
    got = (int(xs[rows[0]]), int(ys[rows[0]]))
    if got != want:
        return [(f"check.{cid}", f"{args['name']} at {want} mm", f"{got} mm")]
    return []


# --- changes_inside, changes_absent -----------------------------------------------------------


def _entries(work: Path, path: str) -> list[dict[str, Any]] | None:
    report = _read_json(work / path)
    entries = report.get("entries") if isinstance(report, dict) else None
    if not isinstance(entries, list):
        return None
    return [e for e in entries if isinstance(e, dict)]


def _plan_mm(pos: Any) -> tuple[int, int] | None:
    if isinstance(pos, dict) and isinstance(pos.get("at"), list) and len(pos["at"]) >= 2:
        return _mm(pos["at"][0]), _mm(pos["at"][1])
    return None


def changes_inside(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """At least one entry whose name starts with `prefix` has a `before` or `after` origin at
    distance <= `radius` from `at`; every such entry's `what` equals `what`, and one of its causes
    includes the members of `cause` (identity.md 4.3, 5: what a hand edit removed, and why)."""
    entries = _entries(work, args["path"])
    if entries is None:
        return [(f"check.{cid}", "a change report with entries", "no `entries` list")]
    ax, ay = _mm(args["at"][0]), _mm(args["at"][1])
    rad = Fraction(args["radius"]) * 1000
    mine = []
    for e in entries:
        if not str(e.get("name", "")).startswith(args["prefix"]):
            continue
        for side in ("before", "after"):
            p = _plan_mm(e.get(side))
            if p is not None and (p[0] - ax) ** 2 + (p[1] - ay) ** 2 <= rad * rad:
                mine.append(e)
                break
    where = f"{args['prefix']}* within {args['radius']} m of {args['at']}"
    if not mine:
        return [(f"check.{cid}", f"an entry {where}", "none")]
    fails: list[Failure] = []
    for e in mine:
        if e.get("what") != args["what"]:
            fails.append((f"check.{cid}", f"every entry {where} is {canon.show(args['what'])}",
                          f"{e.get('name')} is {canon.show(e.get('what'))}"))
            break
        causes = e.get("causes")
        if not isinstance(causes, list) or not any(_members_include(c, args["cause"]) for c in causes):
            fails.append((f"check.{cid}", f"every entry {where} has a cause {canon.show(args['cause'])}",
                          f"{e.get('name')}: {canon.show(causes)}"))
            break
    return fails


def changes_absent(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """No entry is named one of `entry_names`, and no entry has a cause starting with one of `causes`."""
    entries = _entries(work, args["path"])
    if entries is None:
        return [(f"check.{cid}", "a change report with entries", "no `entries` list")]
    fails: list[Failure] = []
    names = set(args.get("entry_names", []))
    hit = [e for e in entries if e.get("name") in names]
    if hit:
        fails.append((f"check.{cid}", f"no entry named {', '.join(sorted(names))}",
                      f"{hit[0].get('table')} {hit[0].get('name')}"))
    prefixes = tuple(args.get("causes", []))
    if prefixes:
        for e in entries:
            causes = e.get("causes") if isinstance(e.get("causes"), list) else []
            bad = [c.get("cause") for c in causes if isinstance(c, dict)
                   and isinstance(c.get("cause"), str) and c["cause"].startswith(prefixes)]
            if bad:
                fails.append((f"check.{cid}", f"no cause starting {', '.join(prefixes)}",
                              f"{e.get('name')}: {bad[0]}"))
                break
    return fails


# --- kernel guarantees (geometry.md 3.1, 3.2, 3.6; identity.md 47) ----------------------------


def _kread(path: Path) -> dict[str, Any]:
    try:
        return kernel.read(path)
    except kernel.KernelFileError as e:
        raise NamedCheckError(str(e)) from e


def network_valid(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    return kernel.network_valid(cid, _kread(work / args["path"]), _kread(work / args["input"]))


def network_measures(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """A measure, reported as a note and never a failure (geometry.md 6, fixtures/README.md)."""
    note = kernel.network_measures(_kread(work / args["path"]), _kread(work / args["reference"]))
    if "note" in ctx:
        ctx["note"](f"{cid}: {note}")
    return []


def noding_valid(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    bound = Fraction(str(args.get("bound", "1.5")))
    return kernel.noding_valid(cid, _kread(work / args["path"]), _kread(work / args["input"]), bound)


def noding_same(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    return kernel.noding_same(cid, _kread(work / args["first"]), _kread(work / args["second"]))


def names_same(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    return kernel.names_same(cid, _kread(work / args["first"]), _kread(work / args["second"]))


def cells_valid(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    try:
        return kernel.cells_valid(cid, _kread(work / args["path"]), _kread(work / args["input"]),
                                  _kread(work / args["network"]), float(args.get("bound", 1.5)))
    except kernel.KernelFileError as e:
        raise NamedCheckError(str(e)) from e


def roundtrip_point(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    try:
        return kernel.roundtrip_point(cid, _kread(work / args["path"]), _kread(work / args["input"]),
                                      Fraction(str(args.get("bound", 1))))
    except kernel.KernelFileError as e:
        raise NamedCheckError(str(e)) from e


def roundtrip_t(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    try:
        return kernel.roundtrip_t(cid, _kread(work / args["path"]), _kread(work / args["input"]),
                                  float(args.get("bound", 1)))
    except kernel.KernelFileError as e:
        raise NamedCheckError(str(e)) from e


def stroke_measures(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """A measure, reported as a note and never a failure (geometry.md 6, fixtures/README.md)."""
    note = kernel.stroke_measures(_kread(work / args["path"]), _kread(work / args["reference"]),
                                  float(args["distance"]))
    if "note" in ctx:
        ctx["note"](f"{cid}: {note}")
    return []


# --- id_rule (build.md 12) ------------------------------------------------------------------------


def id_digest(manifest: dict[str, Any]) -> str:
    """build.md 12: the BLAKE3 digest of the manifest less `id`, `run` and `locations`, as canonical
    JSON text without its closing line feed (the form the null vector's `id.txt` fixes)."""
    rest = {k: v for k, v in manifest.items() if k not in ("id", "run", "locations")}
    return blake3.digest(canon.canonical_bytes(rest)[:-1])


def id_rule(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    path = work / args["build"] / "manifest.json"
    raw = path.read_bytes()
    m = _read_json(path)
    name = f"check.{cid}"
    if not isinstance(m, dict) or not isinstance(m.get("id"), str):
        return [(name, "manifest.json has an id", canon.show(m))]
    if canon.canonical_bytes(m) != raw:
        return [(name, "manifest.json is canonical (documents.md 8)", "it is not")]
    got = id_digest(m)
    if got != m["id"]:
        return [(name, f"id equals the digest of the manifest less id, run and locations ({got})", m["id"])]
    return []


# --- sample_terrain (surfaces.md 2.2, fields.md 1.2) ------------------------------------------


def sample_terrain(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """Each row of a `sample` answer on the terrain equals surfaces.md 2.2 computed from the built
    field's posts: the cell of fields.md 1.2, z its bilinear interpolant, gx and gy that
    interpolant's gradient, in f64 in the chapter's order, bit for bit; `inside` true."""
    name = f"check.{cid}"
    ans = _read_json(work / args["path"])
    inp = _read_json(work / args["input"])
    x0, y0, r, nx, ny = _frame(_read_json(work / args["manifest"]))  # metres (build.md 5)
    x0, y0, r = x0 * 1000, y0 * 1000, r * 1000
    if r.denominator != 1 or x0.denominator != 1 or y0.denominator != 1:
        raise NamedCheckError("manifest frame not in integer millimetres")
    arr = files.load_array(work / args["field"])
    if arr.shape != (ny, nx):
        return [(name, f"field of shape ({ny}, {nx})", str(arr.shape))]
    rm, R = int(r), int(r) / 1000.0
    fails: list[Failure] = []
    for k, (x, y) in enumerate(zip(inp["x"], inp["y"])):
        a, b = x - int(x0), y - int(y0)
        i, j = min(a // rm, nx - 2), min(b // rm, ny - 2)
        fx, fy = (a - i * rm) / rm, (b - j * rm) / rm
        z00, z10 = float(arr[j, i]), float(arr[j, i + 1])
        z01, z11 = float(arr[j + 1, i]), float(arr[j + 1, i + 1])
        s_ = (1 - fx) * z00 + fx * z10
        n_ = (1 - fx) * z01 + fx * z11
        want = {
            "inside": True,
            "z": (1 - fy) * s_ + fy * n_,
            "gx": ((z10 - z00) * (1 - fy) + (z11 - z01) * fy) / R,
            "gy": ((z01 - z00) * (1 - fx) + (z11 - z10) * fx) / R,
        }
        for col, w in want.items():
            try:
                got = ans[col][k]
            except (KeyError, IndexError, TypeError):
                fails.append((name, f"row {k}: `{col}`", "absent"))
                continue
            same = got is w if isinstance(w, bool) else (isinstance(got, (int, float)) and not isinstance(got, bool)
                                                          and float(got) == w and math.copysign(1, float(got)) == math.copysign(1, w))
            if not same:
                fails.append((name, f"row {k} ({x}, {y}) mm, cell ({i}, {j}): {col} = {w!r}", canon.show(got)))
    return fails


def course_valid(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """`derived.md` 3 step 6: the feature named `feature` in a `features.json` is a path whose
    first point is `start` and last is `end` (metres, exactly), that passes each of `passes`, that
    turns by no more than 90 degrees at any vertex, and whose every segment is at least
    `min_segment` metres long."""
    doc = _read_json(work / args["path"])
    feats = [f for f in doc.get("features", []) if f.get("name") == args["feature"]]
    if len(feats) != 1:
        return [(f"check.{cid}", f"one feature {args['feature']}", str(len(feats)))]
    pts = [(Fraction(str(p[0])), Fraction(str(p[1]))) for p in feats[0].get("points", [])]
    want = lambda p: (Fraction(str(p[0])), Fraction(str(p[1])))
    fails: list[Failure] = []
    if not pts or pts[0] != want(args["start"]):
        fails.append((f"check.{cid}", f"first point {args['start']}", canon.show([float(v) for v in pts[0]] if pts else None)))
    if not pts or pts[-1] != want(args["end"]):
        fails.append((f"check.{cid}", f"last point {args['end']}", canon.show([float(v) for v in pts[-1]] if pts else None)))
    for p in args.get("passes", []):
        if want(p) not in pts:
            fails.append((f"check.{cid}", f"passes {p}", "not a vertex"))
    m2 = Fraction(str(args["min_segment"])) ** 2
    for k in range(1, len(pts)):
        d2 = (pts[k][0] - pts[k - 1][0]) ** 2 + (pts[k][1] - pts[k - 1][1]) ** 2
        if d2 < m2:
            fails.append((f"check.{cid}", f"segment {k} at least {args['min_segment']} m", f"{float(d2) ** 0.5:.3f} m"))
    for k in range(1, len(pts) - 1):
        a, b, c = pts[k - 1], pts[k], pts[k + 1]
        dot = (b[0] - a[0]) * (c[0] - b[0]) + (b[1] - a[1]) * (c[1] - b[1])
        if dot < 0:
            fails.append((f"check.{cid}", f"no turn over 90 degrees at vertex {k}", f"({float(b[0])}, {float(b[1])})"))
    return fails


def entry_causes(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """`build.md` 11.2, `graph.md` 6.4: at least one entry of `table` in a `changes.json` has a name
    starting with `prefix`, and every such entry has causes (never `[]` or `null`) among which is
    each cause of `named`."""
    doc = _read_json(work / args["path"])
    hits = [e for e in doc.get("entries", []) if e.get("table") == args["table"]
            and str(e.get("name", "")).startswith(args["prefix"])]
    if not hits:
        return [(f"check.{cid}", f"an entry of {args['table']} named {args['prefix']}...", "none")]
    fails: list[Failure] = []
    for e in hits:
        causes = e.get("causes")
        if not causes:
            fails.append((f"check.{cid}", f"{e['name']} has causes", canon.show(causes)))
            continue
        got = {c.get("cause") for c in causes}
        for want in args.get("named", []):
            if want not in got:
                fails.append((f"check.{cid}", f"{e['name']} has cause {want}", ", ".join(sorted(got))))
    return fails[:5]


def post_step_at_most(cid: str, args: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    """Between every two neighbouring posts (east or north) with both inside `window` (metres,
    `[x0, y0, x1, y1]`, edges included), a scalar field file's values differ by at most `limit`."""
    a = np.load(work / args["path"]).astype(np.float64)
    if a.ndim != 2:
        raise NamedCheckError(f"{args['path']} is not a scalar field file")
    r = float(args["resolution"])
    ox, oy = (float(v) for v in args.get("origin", [0, 0]))
    x0, y0, x1, y1 = (float(v) for v in args["window"])
    i0, i1 = max(0, int(np.ceil((x0 - ox) / r))), min(a.shape[1] - 1, int(np.floor((x1 - ox) / r)))
    j0, j1 = max(0, int(np.ceil((y0 - oy) / r))), min(a.shape[0] - 1, int(np.floor((y1 - oy) / r)))
    if i1 <= i0 or j1 <= j0:
        raise NamedCheckError("the window holds fewer than two posts on an axis")
    w = a[j0:j1 + 1, i0:i1 + 1]
    worst, at = 0.0, None
    for axis, (di, dj) in ((1, (1, 0)), (0, (0, 1))):
        d = np.abs(np.diff(w, axis=axis))
        k = np.unravel_index(int(np.argmax(d)), d.shape)
        if d[k] > worst:
            worst, at = float(d[k]), ((i0 + k[1]) * r + ox, (j0 + k[0]) * r + oy, di, dj)
    if worst > float(args["limit"]):
        return [(f"check.{cid}", f"no step between neighbouring posts in {args['window']} exceeds {args['limit']}",
                 f"{worst:.4f} from the post at ({at[0]}, {at[1]}) to its neighbour ({at[2]}, {at[3]}) posts on")]
    return []


CHECKS: dict[str, Callable[[str, dict[str, Any], Path, dict[str, Any]], list[Failure]]] = {
    "weights_sum_at_most": weights_sum_at_most,
    "weights_sum_255_or_0": weights_sum_255_or_0,
    "no_absolute_path": no_absolute_path,
    "changed_within_windows": changed_within_windows,
    "no_origin_inside": no_origin_inside,
    "changes_by_prefix": changes_by_prefix,
    "vectors_match": vectors_match,
    "json_count": json_count,
    "run_node": run_node,
    "array_window": array_window,
    "run_report_valid": run_report_valid,
    "pack_root_resolves": pack_root_resolves,
    "json_list": json_list,
    "instance_at": instance_at,
    "changes_inside": changes_inside,
    "changes_absent": changes_absent,
    "network_valid": network_valid,
    "network_measures": network_measures,
    "noding_valid": noding_valid,
    "noding_same": noding_same,
    "names_same": names_same,
    "cells_valid": cells_valid,
    "sample_terrain": sample_terrain,
    "course_valid": course_valid,
    "entry_causes": entry_causes,
    "post_step_at_most": post_step_at_most,
    "roundtrip_point": roundtrip_point,
    "roundtrip_t": roundtrip_t,
    "stroke_measures": stroke_measures,
    "id_rule": id_rule,
    "names_count": names_count,
    "names_kept": names_kept,
}


def run_named(cid: str, table: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    args = {k: v for k, v in table.items() if k != "fn"}
    try:
        return CHECKS[table["fn"]](cid, args, work, ctx)
    except NamedCheckError as e:
        return [(f"check.{cid}", f"{table['fn']} can read its inputs", str(e))]
    except (OSError, ValueError, KeyError, TypeError, IndexError) as e:
        return [(f"check.{cid}", f"{table['fn']} can read its inputs", f"{type(e).__name__}: {e}")]
