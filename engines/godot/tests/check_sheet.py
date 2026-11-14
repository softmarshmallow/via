"""Checks what the sheet tool wrote (spec/build.md 15.1), Python's standard library only.

    python3 engines/godot/tests/check_sheet.py OUT ROWS [--one] [--before DIR] [--after DIR]

Every PNG it names is 1280 by 720 and not one colour; each bookmark's eye and target are given per
build; with the build directories, each cell's eye and target are its own build's bookmark (its
`at` raised by `eye_height`, and its `toward`), so the camera of the before cell looks at the before
build's target and the after cell at the after build's;
sheet.png is 1280 (640 with --one) by
48 + 384 * ROWS, walks.png by 48 + 768 * ROWS, each walk's two views 1280 by 720; sheet.json is canonical and has the keys 15.1 lists; diagnostics.json is
diagnostics.md 7's document. Images are never compared byte for byte: renderers differ.
Prints one line per check; exit 0 when all pass.
"""

import json
import struct
import sys
import zlib
from pathlib import Path


def png(path):
    """(width, height, rows of RGB(A) bytes) of an 8-bit truecolour PNG."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, idat, w = 8, b"", 0
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, ctype = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and ctype in (2, 6), f"colour type {ctype}, depth {depth}"
            bpp = 3 if ctype == 2 else 4
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * bpp
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return w, h, rows, bpp


def _norm(v):
    """Integral doubles as integers, as canonical JSON writes them (documents.md 8)."""
    if isinstance(v, float) and v == int(v):
        return int(v)
    if isinstance(v, dict):
        return {k: _norm(x) for k, x in v.items()}
    if isinstance(v, list):
        return [_norm(x) for x in v]
    return v


def bookmarks(build):
    """{name: (eye, toward)} of a build's bookmark outputs, as the loader reads them."""
    if build is None:
        return {}
    build = Path(build)
    manifest = json.loads((build / "manifest.json").read_text())
    feats = json.loads((build / (manifest.get("features") or "features.json")).read_text())["features"]
    by_name = {f["name"]: f for f in feats}
    out = {}
    for n in manifest.get("outputs", {}).get("bookmark", []):
        f = by_name.get(n, {})
        attrs = f.get("attrs") or {}
        at, h = f.get("at"), attrs.get("eye_height")
        if isinstance(at, list) and len(at) == 3 and isinstance(h, (int, float)):
            out[n] = ([at[0], at[1], at[2] + h], attrs.get("toward"))
    return out


def close(a, b, tol=1e-6):
    """Two points (or targets) equal to within ``tol`` per coordinate: sheet.json carries
    the loader's doubles, written canonically, so a last-bit difference is allowed."""
    if isinstance(a, dict) and isinstance(b, dict):
        return a.get("name") == b.get("name") and close(a.get("at"), b.get("at"), tol)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(abs(float(x) - float(y)) <= tol for x, y in zip(a, b))
    return False


def one_colour(rows, bpp):
    first = rows[0][:bpp]
    return all(r == first * (len(r) // bpp) for r in rows)


def main():
    out, n_rows, one = Path(sys.argv[1]), int(sys.argv[2]), "--one" in sys.argv[3:]
    rest = sys.argv[3:]
    builds = {side: rest[rest.index("--" + side) + 1] if "--" + side in rest else None
              for side in ("before", "after")}
    own = {side: bookmarks(builds[side]) for side in builds}
    failed = 0

    def check(what, ok, detail=""):
        nonlocal failed
        failed += not ok
        print(("ok   " if ok else "FAIL ") + what + (f": {detail}" if detail else ""))

    text = (out / "sheet.json").read_text()
    doc = json.loads(text)
    canon = json.dumps(_norm(doc), sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"
    check("sheet.json is canonical", text == canon)
    check("sheet.json keys", sorted(doc) == ["after", "before", "bookmarks", "camera", "edits", "godot", "light", "renderer", "size", "walk", "walks"])
    check("edits lists pins and cleared", isinstance(doc.get("edits"), dict) and sorted(doc["edits"]) == ["against", "cleared", "pins"])
    check("one walk per bookmark, in name order", [w["name"] for w in doc.get("walks", [])] == [b["name"] for b in doc["bookmarks"]])
    walk_keys = ["by", "deepest", "drop", "eye", "from", "left", "outcome", "plan", "slope", "stop", "through", "toward", "walked"]
    for w in doc.get("walks", []):
        for side in ("before", "after"):
            r = w[side]
            if r is None:
                continue
            check(f"walk {w['name']} {side}: a report", sorted(r) == walk_keys
                  and r["outcome"] in ("arrived", "blocked", "fell", "timed_out"), str(r.get("outcome")))
            for view in ("plan", "eye"):
                ww, hh, rr, bb = png(out / r[view])
                check(f"{r[view]} is 1280 by 720 and not one colour", (ww, hh) == (1280, 720) and not one_colour(rr, bb), f"{ww}x{hh}")
    check("one entry per row, in name order", [b["name"] for b in doc["bookmarks"]] == sorted(b["name"] for b in doc["bookmarks"])
          and len(doc["bookmarks"]) == n_rows)
    check("after is null without --after", (doc["after"] is None) == one)
    for b in doc["bookmarks"]:
        eye = b["eye"]
        check(f"{b['name']}: an eye per build, where it has an image", isinstance(eye, dict) and sorted(eye) == ["after", "before"]
              and all((eye[side] is None) == (b[side] is None) for side in ("before", "after"))
              and all(len(e) == 3 for e in eye.values() if e is not None), str(eye))
        target = b.get("target")
        check(f"{b['name']}: a target per build, where it has an image", isinstance(target, dict)
              and sorted(target) == ["after", "before"]
              and all((target[side] is None) == (b[side] is None) for side in ("before", "after"))
              and all(isinstance(t, dict) and sorted(t) == ["at", "name"] and len(t["at"]) == 3
                      for t in target.values() if t is not None), str(target))
        for side in ("before", "after"):
            if builds[side] is None or b[side] is None or not isinstance(target, dict):
                continue
            want = own[side].get(b["name"])
            check(f"{b['name']}: the {side} cell looks from its own build's eye at its own build's target",
                  want is not None and close(eye[side], want[0]) and close(target[side], want[1]),
                  f"recorded eye {eye[side]} target {target[side]}, the {side} build's {want}")
        for side in ("before", "after"):
            if b[side] is None:
                continue
            w, h, rows, bpp = png(out / b[side])
            check(f"{b[side]} is 1280 by 720 and not one colour", (w, h) == (1280, 720) and not one_colour(rows, bpp), f"{w}x{h}")
    w, h, rows, bpp = png(out / "sheet.png")
    want = (640 if one else 1280, 48 + 384 * n_rows)
    check(f"sheet.png is {want[0]} by {want[1]}", (w, h) == want, f"{w}x{h}")
    w, h, rows, bpp = png(out / "walks.png")
    want = (640 if one else 1280, 48 + 384 * 2 * n_rows)
    check(f"walks.png is {want[0]} by {want[1]}", (w, h) == want, f"{w}x{h}")
    diag = json.loads((out / "diagnostics.json").read_text())
    check("diagnostics.json is diagnostics.md 7's document", diag.get("format") == "via-diagnostics-1"
          and set(diag) == {"counts", "diagnostics", "format", "lints", "status"})
    print("PASS" if not failed else f"FAIL: {failed} check(s) failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
