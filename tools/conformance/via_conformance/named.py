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

from . import canon, files

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


CHECKS: dict[str, Callable[[str, dict[str, Any], Path, dict[str, Any]], list[Failure]]] = {
    "weights_sum_at_most": weights_sum_at_most,
    "weights_sum_255_or_0": weights_sum_255_or_0,
    "no_absolute_path": no_absolute_path,
    "changed_within_windows": changed_within_windows,
    "no_origin_inside": no_origin_inside,
    "changes_by_prefix": changes_by_prefix,
}


def run_named(cid: str, table: dict[str, Any], work: Path, ctx: dict[str, Any]) -> list[Failure]:
    args = {k: v for k, v in table.items() if k != "fn"}
    try:
        return CHECKS[table["fn"]](cid, args, work, ctx)
    except NamedCheckError as e:
        return [(f"check.{cid}", f"{table['fn']} can read its inputs", str(e))]
    except (OSError, ValueError, KeyError, TypeError, IndexError) as e:
        return [(f"check.{cid}", f"{table['fn']} can read its inputs", f"{type(e).__name__}: {e}")]
